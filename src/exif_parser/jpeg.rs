use super::generic::{get_datetime_from_exif, get_gps_coord};
use super::gps_parser;
use anyhow::Result;
use chrono::{DateTime, Utc};
use exif::Tag;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn extract_metadata_from_jpeg(path: &Path) -> Result<(f64, f64, Option<DateTime<Utc>>)> {
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    let mut exif_reader = exif::Reader::new();
    exif_reader.continue_on_error(true); // Tolerate non-standard EXIF structures
    
    match exif_reader.read_from_container(&mut buf_reader) {
        Ok(exif) => {
            // Try to extract GPS using standard method
            if let (Some(lat), Some(lng)) = (
                get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?,
                get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?,
            ) {
                let datetime = get_datetime_from_exif(&exif);
                return Ok((lat, lng, datetime));
            }
        }
        Err(exif::Error::PartialResult(partial)) => {
            let (exif, _errors) = partial.into_inner();
            // Try to extract GPS from partial result
            if let (Some(lat), Some(lng)) = (
                get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?,
                get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?,
            ) {
                let datetime = get_datetime_from_exif(&exif);
                return Ok((lat, lng, datetime));
            }
        }
        Err(_) => {}
    }
    
    // Fallback to custom GPS parser for malformed EXIF files (e.g., Lightroom-processed)
    if let Some((lat, lng)) = gps_parser::extract_gps_from_malformed_exif(path) {
        // We have GPS, but no datetime from custom parser
        // Try to get datetime from standard EXIF if possible
        let datetime = File::open(path)
            .ok()
            .and_then(|f| {
                let mut buf = BufReader::new(f);
                exif::Reader::new().read_from_container(&mut buf).ok()
            })
            .and_then(|exif| get_datetime_from_exif(&exif));
        
        return Ok((lat, lng, datetime));
    }

    anyhow::bail!("GPS data not found in JPEG file")
}
