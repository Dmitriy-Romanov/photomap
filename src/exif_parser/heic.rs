use anyhow::Result;
use exif::Tag;
use std::path::Path;
use super::generic::{get_gps_coord, get_datetime_from_exif};

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
