use anyhow::Result;
use exif::{In, Reader, Tag, Value};
use std::fs;
use std::path::Path;

/// Применяет EXIF-ориентацию к изображению
pub fn apply_exif_orientation(source_path: &Path, img: image::DynamicImage) -> Result<image::DynamicImage> {
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

pub fn get_gps_coord(
    exif: &exif::Exif,
    coord_tag: Tag,
    ref_tag: Tag,
) -> Result<Option<f64>> {
    let coord_field = exif.get_field(coord_tag, In::PRIMARY);
    let ref_field = exif.get_field(ref_tag, In::PRIMARY);

    if let (Some(coord), Some(ref_val)) = (coord_field, ref_field) {
        if let Value::Rational(ref vec) = coord.value {
            if vec.len() == 3 {
                let d = vec[0].to_f64();
                let m = vec[1].to_f64();
                let s = vec[2].to_f64();
                let mut decimal = d + (m / 60.0) + (s / 3600.0);

                // Применяем референс (S/W - отрицательные значения)
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

pub fn get_datetime_from_exif(exif: &exif::Exif) -> Option<String> {
    // Сначала пробуем стандартный тег DateTimeOriginal (если он есть),
    // затем пробуем более общий тег DateTime.
    let try_tags = [Tag::DateTimeOriginal, Tag::DateTime];

    for &tag in &try_tags {
        if let Some(field) = exif.get_field(tag, In::PRIMARY) {
            if let exif::Value::Ascii(ref vec) = field.value {
                if let Some(datetime_str) = vec.first() {
                    // Формат EXIF обычно: "YYYY:MM:DD HH:MM:SS"
                    if let Ok(s) = std::str::from_utf8(datetime_str) {
                        let parts: Vec<&str> = s.split(' ').collect();
                        if parts.len() == 2 {
                            let date_parts: Vec<&str> = parts[0].split(':').collect();
                            let time_parts: Vec<&str> = parts[1].split(':').collect();

                            if date_parts.len() == 3 && time_parts.len() >= 2 {
                                let year = date_parts[0];
                                let month = date_parts[1];
                                let day = date_parts[2];
                                let hour = time_parts[0];
                                let min = time_parts[1];

                                return Some(format!("Дата съемки: {}.{}.{} {}:{}", day, month, year, hour, min));
                            }
                        }
                    }
                }
            }
        }
    }

    None
}
