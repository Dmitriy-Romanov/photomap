pub mod generic;
pub mod gps_parser;
pub mod heic;
pub mod jpeg;

pub use generic::{apply_exif_orientation, get_datetime_string, get_gps_coord};
pub use heic::extract_metadata_from_heic;
pub use jpeg::extract_metadata_from_jpeg;

#[derive(Debug, thiserror::Error)]
pub enum ExifError {
    #[error("GPS data not found")]
    GpsNotFound,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("EXIF error: {0}")]
    Exif(#[from] exif::Error),
}
