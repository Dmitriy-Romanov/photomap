use super::generic::{get_datetime_from_exif, get_gps_coord};
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use exif::Tag;
use libheif_rs::ItemId;
use std::path::Path;

pub fn extract_metadata_from_heic(path: &Path) -> Result<(f64, f64, Option<DateTime<Utc>>)> {
    let ctx = libheif_rs::HeifContext::read_from_file(path.to_str().unwrap())
        .map_err(|e| anyhow::anyhow!("Failed to read HEIF context: {}", e))?;

    let primary_image_handle = ctx
        .primary_image_handle()
        .map_err(|e| anyhow::anyhow!("Failed to get primary image handle: {}", e))?;

    // Corrected usage for metadata_block_ids based on compiler's implied signature
    let mut metadata_ids_buffer: [ItemId; 10] = [0; 10]; // Pre-allocate a buffer for up to 10 ItemIds
    let count = primary_image_handle.metadata_block_ids(&mut metadata_ids_buffer[..], b"Exif");

    if count == 0 {
        bail!("No Exif metadata found in HEIF file");
    }

    for id in metadata_ids_buffer.iter().take(count) {
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

    bail!("GPS data not found in HEIF file")
}
