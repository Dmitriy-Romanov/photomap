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

// Native HEIC parser without external libraries
pub fn extract_metadata_from_heif_custom(path: &Path) -> Result<(f64, f64, String)> {
    let data = std::fs::read(path)?;

    // Ищем начало EXIF данных в HEIC файле
    // EXIF обычно хранится после "Exif" маркера
    let mut exif_start = None;

    // Ищем последовательность байт "Exif" в файле
    for i in 0..data.len().saturating_sub(4) {
        if data[i] == b'E' && data[i+1] == b'x' && data[i+2] == b'i' && data[i+3] == b'f' {
            // Пропускаем "Exif" и 2 байта после него
            exif_start = Some(i + 6);
            break;
        }
    }

    if let Some(start) = exif_start {
        // Ищем начало TIFF данных (II или MM)
        let mut tiff_start = start;
        while tiff_start < data.len().saturating_sub(1) {
            if (data[tiff_start] == b'I' && data[tiff_start + 1] == b'I') ||
               (data[tiff_start] == b'M' && data[tiff_start + 1] == b'M') {
                break;
            }
            tiff_start += 1;
        }

        if tiff_start < data.len().saturating_sub(1) {
            // Используем стандартную библиотеку exif для парсинга найденных данных
            if let Ok(exif) = exif::Reader::new().read_raw(data[tiff_start..].to_vec()) {
                let lat = get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?;
                let lng = get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?;
                let datetime = get_datetime_from_exif(&exif).unwrap_or_else(|| "Дата неизвестна".to_string());

                if lat.is_some() && lng.is_some() {
                    return Ok((lat.unwrap(), lng.unwrap(), datetime));
                }
            }
        }
    }

    anyhow::bail!("GPS-данные не найдены в HEIF файле")
}

// Собственный парсер JPEG без сторонних библиотек
pub fn extract_metadata_from_jpeg_custom(path: &Path) -> Result<(f64, f64, String)> {
    let data = std::fs::read(path)?;

    // Ищем EXIF сегмент в JPEG файле
    // EXIF хранится в APP1 сегменте (FF E1)
    let mut i = 2; // Начинаем после SOI маркера (FF D8)

    while i < data.len().saturating_sub(4) {
        // Ищем начало следующего сегмента (маркер FF)
        if data[i] != 0xFF {
            i += 1;
            continue;
        }

        let marker = data[i+1];

        // APP1 сегмент
        if marker == 0xE1 {
            let segment_length = ((data[i+2] as u16) << 8) | (data[i+3] as u16);
            let segment_end = i + segment_length as usize + 2;

            // Проверяем, что это EXIF сегмент
            if i + 10 < data.len() &&
               &data[i+4..i+10] == b"Exif\0\0" {

                let exif_start = i + 10;

                // Проверяем наличие TIFF заголовка
                if exif_start + 2 < data.len() &&
                   ((data[exif_start] == b'I' && data[exif_start + 1] == b'I') ||
                    (data[exif_start] == b'M' && data[exif_start + 1] == b'M')) {

                    // Используем стандартную библиотеку exif для парсинга
                    if let Ok(exif) = exif::Reader::new().read_raw(data[exif_start..segment_end].to_vec()) {
                        let lat = get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?;
                        let lng = get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?;
                        let datetime = get_datetime_from_exif(&exif).unwrap_or_else(|| "Дата неизвестна".to_string());

                        if lat.is_some() && lng.is_some() {
                            return Ok((lat.unwrap(), lng.unwrap(), datetime));
                        }
                    }
                }
            }
            // Переходим к следующему сегменту
            i += segment_length as usize + 2;
        } else if (marker >= 0xE0 && marker <= 0xEF) || marker == 0xDB || marker == 0xC4 || marker == 0xC0 {
            // Другие сегменты с длиной (APPn, DQT, DHT, SOF0)
            let segment_length = ((data[i+2] as u16) << 8) | (data[i+3] as u16);
            i += segment_length as usize + 2;
        } else if marker == 0xDA { // SOS (Start of Scan)
            // После SOS идут данные изображения до следующего маркера, просто выходим
            break;
        }
        else {
            i += 1;
        }
    }

    anyhow::bail!("GPS-данные не найдены в JPEG файле")
}