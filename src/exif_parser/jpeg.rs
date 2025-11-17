use anyhow::Result;
use exif::Tag;
use std::path::Path;
use super::generic::{get_gps_coord, get_datetime_from_exif};

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
