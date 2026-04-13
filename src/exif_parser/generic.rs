use anyhow::Result;
use exif::{In, Reader, Tag, Value};
use std::fs;
use std::path::Path;

/// Parses EXIF datetime string (format: "YYYY:MM:DD HH:MM:SS") into ISO format
/// Returns None if parsing fails
pub fn parse_exif_datetime(s: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(s).ok()?;
    if s.len() >= 19 {
        // Format: "YYYY:MM:DD HH:MM:SS"
        let parts: Vec<&str> = s.split(' ').collect();
        if parts.len() == 2 {
            let date_parts: Vec<&str> = parts[0].split(':').collect();
            let time_parts: Vec<&str> = parts[1].split(':').collect();
            if date_parts.len() == 3 && time_parts.len() == 3 {
                return Some(format!(
                    "{}-{}-{} {}:{}:{}",
                    date_parts[0], date_parts[1], date_parts[2],
                    time_parts[0], time_parts[1], time_parts[2]
                ));
            }
        }
    }
    None
}

/// Extracts datetime string from EXIF data
pub fn get_datetime_string(exif: &exif::Exif) -> Option<String> {
    let try_tags = [Tag::DateTimeOriginal, Tag::DateTime];

    for &tag in &try_tags {
        if let Some(field) = exif.get_field(tag, In::PRIMARY) {
            if let exif::Value::Ascii(ref vec) = field.value {
                if let Some(datetime_bytes) = vec.first() {
                    if let Some(dt) = parse_exif_datetime(datetime_bytes) {
                        return Some(dt);
                    }
                }
            }
        }
    }
    None
}

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

/// Validate that a float value is safe to use (not NaN or Infinity)
fn is_valid_float(value: f64) -> bool {
    !value.is_nan() && !value.is_infinite()
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

                            // Validate all components before calculation
                            if !is_valid_float(d) || !is_valid_float(m) || !is_valid_float(s) {
                                continue;
                            }

                            let mut decimal = d + (m / 60.0) + (s / 3600.0);

                            // Validate final result
                            if !is_valid_float(decimal) {
                                continue;
                            }

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

                            // Validate all components before calculation
                            if !is_valid_float(d) || !is_valid_float(m) || !is_valid_float(s) {
                                continue;
                            }

                            let mut decimal = d + (m / 60.0) + (s / 3600.0);

                            // Validate final result
                            if !is_valid_float(decimal) {
                                continue;
                            }

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

                // Validate all components before calculation
                if !is_valid_float(d) || !is_valid_float(m) || !is_valid_float(s) {
                    return Ok(None);
                }

                let mut decimal = d + (m / 60.0) + (s / 3600.0);

                // Validate final result
                if !is_valid_float(decimal) {
                    return Ok(None);
                }

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

                // Validate all components before calculation
                if !is_valid_float(d) || !is_valid_float(m) || !is_valid_float(s) {
                    return Ok(None);
                }

                let mut decimal = d + (m / 60.0) + (s / 3600.0);

                // Validate final result
                if !is_valid_float(decimal) {
                    return Ok(None);
                }

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
