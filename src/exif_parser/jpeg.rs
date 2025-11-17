use anyhow::Result;
use exif::Tag;
use std::path::Path;
use super::generic::{get_gps_coord, get_datetime_from_exif};

// Custom JPEG parser without external libraries
pub fn extract_metadata_from_jpeg_custom(path: &Path) -> Result<(f64, f64, String)> {
    let data = std::fs::read(path)?;

    // Search for the EXIF segment in the JPEG file
    // EXIF is stored in the APP1 segment (FF E1)
    let mut i = 2; // Start after the SOI marker (FF D8)

    while i < data.len().saturating_sub(4) {
        // Search for the start of the next segment (FF marker)
        if data[i] != 0xFF {
            i += 1;
            continue;
        }

        let marker = data[i+1];

        // APP1 segment
        if marker == 0xE1 {
            let segment_length = ((data[i+2] as u16) << 8) | (data[i+3] as u16);
            let segment_end = i + segment_length as usize + 2;

            // Check if this is an EXIF segment
            if i + 10 < data.len() &&
               &data[i+4..i+10] == b"Exif\0\0" {

                let exif_start = i + 10;

                // Check for TIFF header
                if exif_start + 2 < data.len() &&
                   ((data[exif_start] == b'I' && data[exif_start + 1] == b'I') ||
                    (data[exif_start] == b'M' && data[exif_start + 1] == b'M')) {

                    // Use the standard exif library for parsing
                    if let Ok(exif) = exif::Reader::new().read_raw(data[exif_start..segment_end].to_vec()) {
                        let lat = get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?;
                        let lng = get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?;
                        let datetime = get_datetime_from_exif(&exif).unwrap_or_else(|| "Date unknown".to_string());

                        if lat.is_some() && lng.is_some() {
                            return Ok((lat.unwrap(), lng.unwrap(), datetime));
                        }
                    }
                }
            }
            // Move to the next segment
            i += segment_length as usize + 2;
        } else if (marker >= 0xE0 && marker <= 0xEF) || marker == 0xDB || marker == 0xC4 || marker == 0xC0 {
            // Other segments with length (APPn, DQT, DHT, SOF0)
            let segment_length = ((data[i+2] as u16) << 8) | (data[i+3] as u16);
            i += segment_length as usize + 2;
        } else if marker == 0xDA { // SOS (Start of Scan)
            // After SOS comes the image data until the next marker, so we just exit
            break;
        }
        else {
            i += 1;
        }
    }

    anyhow::bail!("GPS data not found in JPEG file")
}
