use anyhow::Result;
use exif::Tag;
use std::path::Path;
use super::generic::{get_gps_coord, get_datetime_from_exif};

use anyhow::{Result, bail};
use exif::Tag;
use std::path::Path;
use super::generic::{get_gps_coord, get_datetime_from_exif};

pub fn extract_metadata_from_heic(path: &Path) -> Result<(f64, f64, String)> {
    let ctx = libheif_rs::HeifContext::read_from_file(path.to_str().unwrap())
        .map_err(|e| anyhow::anyhow!("Failed to read HEIF context: {}", e))?;

    let primary_image_handle = ctx.primary_image_handle()
        .map_err(|e| anyhow::anyhow!("Failed to get primary image handle: {}", e))?;

    let metadata_ids = primary_image_handle.metadata_block_ids(b"Exif")
        .map_err(|e| anyhow::anyhow!("Failed to get Exif metadata block IDs: {}", e))?;

    for id in metadata_ids {
        let exif_data = primary_image_handle.metadata(id)
            .map_err(|e| anyhow::anyhow!("Failed to get metadata for ID {}: {}", id, e))?;

        // `libheif-rs` provides the raw EXIF data, which usually starts with "Exif\0\0"
        // and then the TIFF header. `exif::Reader::read_raw` expects the TIFF header directly.
        // We need to skip the "Exif\0\0" part if it's present.
        let tiff_header_start = if exif_data.starts_with(b"Exif\0\0") {
            6
        } else {
            0
        };

        if exif_data.len() > tiff_header_start {
            if let Ok(exif) = exif::Reader::new().read_raw(&exif_data[tiff_header_start..].to_vec()) {
                let lat = get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?;
                let lng = get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?;
                let datetime = get_datetime_from_exif(&exif).unwrap_or_else(|| "Date unknown".to_string());

                if let (Some(lat), Some(lng)) = (lat, lng) {
                    return Ok((lat, lng, datetime));
                }
            }
        }
    }

    bail!("GPS data not found in HEIF file")
}
