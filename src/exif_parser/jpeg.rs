use super::generic::{get_datetime_from_exif, get_gps_coord};
use anyhow::Result;
use chrono::{DateTime, Utc};
use exif::Tag;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn extract_metadata_from_jpeg(path: &Path) -> Result<(f64, f64, Option<DateTime<Utc>>)> {
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    let exif_reader = exif::Reader::new();
    let exif = exif_reader.read_from_container(&mut buf_reader)?;

    let lat = get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?;
    let lng = get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?;
    let datetime = get_datetime_from_exif(&exif);

    if let (Some(lat), Some(lng)) = (lat, lng) {
        Ok((lat, lng, datetime))
    } else {
        anyhow::bail!("GPS data not found in JPEG file")
    }
}
