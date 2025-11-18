use anyhow::Result;
use exif::Tag;
use std::path::Path;
use super::generic::{get_gps_coord, get_datetime_from_exif};

use anyhow::Result;
use exif::Tag;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use super::generic::{get_gps_coord, get_datetime_from_exif};

pub fn extract_metadata_from_jpeg(path: &Path) -> Result<(f64, f64, String)> {
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    let exif_reader = exif::Reader::new();
    let exif = exif_reader.read_from_container(&mut buf_reader)?;

    let lat = get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?;
    let lng = get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?;
    let datetime = get_datetime_from_exif(&exif).unwrap_or_else(|| "Date unknown".to_string());

    if let (Some(lat), Some(lng)) = (lat, lng) {
        Ok((lat, lng, datetime))
    } else {
        anyhow::bail!("GPS data not found in JPEG file")
    }
}
