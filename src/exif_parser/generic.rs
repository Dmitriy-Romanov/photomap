use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use exif::{In, Reader, Tag, Value};
use std::fs;
use std::path::Path;

/// Applies EXIF orientation to the image
pub fn apply_exif_orientation(
    source_path: &Path,
    img: image::DynamicImage,
) -> Result<image::DynamicImage> {
    let file = match fs::File::open(source_path) {
        Ok(f) => f,
        Err(_) => return Ok(img),
    };

    let mut bufreader = std::io::BufReader::new(&file);
    let exifreader = Reader::new();

    let exif = match exifreader.read_from_container(&mut bufreader) {
        Ok(e) => e,
        Err(_) => return Ok(img),
    };

    let orientation = exif
        .get_field(Tag::Orientation, In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(1);

    let rotated = match orientation {
        1 => img,
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate270().fliph(),
        6 => img.rotate90(),
        7 => img.rotate90().fliph(),
        8 => img.rotate270(),
        _ => img,
    };

    Ok(rotated)
}

pub fn get_gps_coord(exif: &exif::Exif, coord_tag: Tag, ref_tag: Tag) -> Result<Option<f64>> {
    // Try PRIMARY IFD first (most common location)
    if let Some(result) = try_get_gps_from_ifd(exif, coord_tag, ref_tag, In::PRIMARY)? {
        return Ok(Some(result));
    }
    
    // Fallback: Search through ALL fields to find GPS data
    // Some cameras (like Samsung) may store GPS in different IFDs or use SRational instead of Rational
    for field in exif.fields() {
        if field.tag == coord_tag {
            // Found coordinate field - now find its reference
            for ref_field in exif.fields() {
                if ref_field.tag == ref_tag && ref_field.ifd_num == field.ifd_num {
                    // Found matching reference in same IFD
                    
                    // Try Rational (unsigned) first - most common
                    if let Value::Rational(ref vec) = &field.value {
                        if vec.len() == 3 {
                            let d = vec[0].to_f64();
                            let m = vec[1].to_f64();
                            let s = vec[2].to_f64();
                            let mut decimal = d + (m / 60.0) + (s / 3600.0);

                            // Apply reference (S/W are negative values)
                            if let Some(ref_val) = ref_field.display_value().to_string().chars().next() {
                                if ref_val == 'S' || ref_val == 'W' {
                                    decimal *= -1.0;
                                }
                            }
                            return Ok(Some(decimal));
                        }
                    }
                    
                    // Try SRational (signed) - some Samsung devices use this (e.g., SM-N900)
                    if let Value::SRational(ref vec) = &field.value {
                        if vec.len() == 3 {
                            let d = vec[0].to_f64();
                            let m = vec[1].to_f64();
                            let s = vec[2].to_f64();
                            let mut decimal = d + (m / 60.0) + (s / 3600.0);

                            // Apply reference (S/W are negative values)
                            if let Some(ref_val) = ref_field.display_value().to_string().chars().next() {
                                if ref_val == 'S' || ref_val == 'W' {
                                    decimal *= -1.0;
                                }
                            }
                            return Ok(Some(decimal));
                        }
                    }
                }
            }
        }
    }
    
    Ok(None)
}

// Helper function to try GPS extraction from specific IFD
fn try_get_gps_from_ifd(exif: &exif::Exif, coord_tag: Tag, ref_tag: Tag, ifd: In) -> Result<Option<f64>> {
    let coord_field = exif.get_field(coord_tag, ifd);
    let ref_field = exif.get_field(ref_tag, ifd);

    if let (Some(coord), Some(ref_val)) = (coord_field, ref_field) {
        // Try Rational (unsigned) first - most common
        if let Value::Rational(ref vec) = coord.value {
            if vec.len() == 3 {
                let d = vec[0].to_f64();
                let m = vec[1].to_f64();
                let s = vec[2].to_f64();
                let mut decimal = d + (m / 60.0) + (s / 3600.0);

                // Apply reference (S/W are negative values)
                if let Some(ref_val) = ref_val.display_value().to_string().chars().next() {
                    if ref_val == 'S' || ref_val == 'W' {
                        decimal *= -1.0;
                    }
                }
                return Ok(Some(decimal));
            }
        }
        
        // Try SRational (signed) - some Samsung devices use this (e.g., SM-N900)
        if let Value::SRational(ref vec) = coord.value {
            if vec.len() == 3 {
                let d = vec[0].to_f64();
                let m = vec[1].to_f64();
                let s = vec[2].to_f64();
                let mut decimal = d + (m / 60.0) + (s / 3600.0);

                // Apply reference (S/W are negative values)
                if let Some(ref_val) = ref_val.display_value().to_string().chars().next() {
                    if ref_val == 'S' || ref_val == 'W' {
                        decimal *= -1.0;
                    }
                }
                return Ok(Some(decimal));
            }
        }
    }
    Ok(None)
}

pub fn get_datetime_from_exif(exif: &exif::Exif) -> Option<DateTime<Utc>> {
    let try_tags = [Tag::DateTimeOriginal, Tag::DateTime];

    for &tag in &try_tags {
        if let Some(field) = exif.get_field(tag, In::PRIMARY) {
            if let exif::Value::Ascii(ref vec) = field.value {
                if let Some(datetime_bytes) = vec.first() {
                    if let Ok(s) = std::str::from_utf8(datetime_bytes) {
                        // EXIF format is usually: "YYYY:MM:DD HH:MM:SS"
                        let s = s.replace(" ", "T"); // Convert to "YYYY:MM:DDTHH:MM:SS"
                        let s = s.replacen(":", "-", 2); // Convert to "YYYY-MM-DD HH:MM:SS"

                        // Parse with NaiveDateTime first, then make it Utc
                        if let Ok(naive_datetime) =
                            NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S")
                        {
                            return Some(DateTime::<Utc>::from_naive_utc_and_offset(
                                naive_datetime,
                                Utc,
                            ));
                        }
                    }
                }
            }
        }
    }

    None
}
