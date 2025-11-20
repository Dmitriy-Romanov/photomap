use super::generic::{get_datetime_from_exif, get_gps_coord};
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use exif::Tag;
use std::path::Path;

pub fn extract_metadata_from_heic(path: &Path) -> Result<(f64, f64, Option<DateTime<Utc>>)> {
    // Try to read as HEIC first
    let heic_result = (|| -> Result<(f64, f64, Option<DateTime<Utc>>)> {
        let ctx = libheif_rs::HeifContext::read_from_file(path.to_str().unwrap())
            .map_err(|e| anyhow::anyhow!("Failed to read HEIF context: {}", e))?;

        let primary_image_handle = ctx
            .primary_image_handle()
            .map_err(|e| anyhow::anyhow!("Failed to get primary image handle: {}", e))?;

        // Corrected usage for metadata_block_ids based on compiler's implied signature
        // Pass 0 for type_filter to match all types (0 implements Into<FourCC>)
        let count = primary_image_handle.number_of_metadata_blocks(0);
        
        if count == 0 {
            bail!("No metadata found in HEIF file");
        }

        let mut metadata_ids_buffer = vec![0; count as usize];
        let count = primary_image_handle.metadata_block_ids(&mut metadata_ids_buffer, 0);

        for id in metadata_ids_buffer.iter().take(count) {
            // Check if it's Exif
            if let Some(type_str) = primary_image_handle.metadata_type(*id) {
                if type_str == "Exif" {
                     let exif_data = primary_image_handle
                        .metadata(*id)
                        .map_err(|e| anyhow::anyhow!("Failed to get metadata for ID {}: {}", id, e))?;

                    // `libheif-rs` provides the raw EXIF data, which usually starts with "Exif\0\0"
                    // and then the TIFF header. `exif::Reader::read_raw` expects the TIFF header directly.
                    // The first 4 bytes are the length of the data, so we skip them.
                    let tiff_header_start = if exif_data.len() > 4 && exif_data[4..].starts_with(b"Exif\0\0") {
                        10
                    } else if exif_data.starts_with(b"Exif\0\0") {
                        6
                    } else {
                        0
                    };

                    if exif_data.len() > tiff_header_start {
                        if let Ok(exif) = exif::Reader::new().read_raw(exif_data[tiff_header_start..].to_vec())
                        {
                            let lat = get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?;
                            let lng = get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?;
                            let datetime = get_datetime_from_exif(&exif);

                            if let (Some(lat), Some(lng)) = (lat, lng) {
                                return Ok((lat, lng, datetime));
                            }
                        }
                    }
                }
            }
        }
        bail!("GPS data not found in HEIF file")
    })();

    if heic_result.is_ok() {
        return heic_result;
    }

    // Fallback: Check if it's actually a JPEG disguised as HEIC (Xiaomi bug)
    use std::fs::File;
    use std::io::Read;
    
    if let Ok(mut file) = File::open(path) {
        let mut buffer = [0u8; 2];
        if file.read_exact(&mut buffer).is_ok() && buffer == [0xFF, 0xD8] {
            // It's a JPEG! Delegate to JPEG parser
            return super::jpeg::extract_metadata_from_jpeg(path);
        }
    }

    heic_result
}
