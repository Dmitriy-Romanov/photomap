// src/constants.rs

pub const MARKER_SIZE: u32 = 40;
pub const THUMBNAIL_SIZE: u32 = 120;  // For map markers and spiderweb (2x for HiDPI)
pub const GALLERY_SIZE: u32 = 240;    // For gallery modal
pub const POPUP_SIZE: u32 = 1400;

/// Checks if a file extension is a supported image format (case-insensitive)
pub fn is_supported_image(ext: &str) -> bool {
    matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "heic" | "heif" | "avif")
}

/// Checks if a file extension is a HEIC/HEIF format (case-insensitive)
pub fn is_heic_format(ext: &str) -> bool {
    matches!(ext.to_lowercase().as_str(), "heic" | "heif" | "avif")
}
