use anyhow::{Context, Result};

use crate::constants::*;
use crate::database::PhotoMetadata;
use image::{DynamicImage, GenericImageView, ImageReader};
use std::path::{Path, PathBuf};

/// Creates a scaled JPG image from a DynamicImage.
/// Can optionally pad the image to a square.
fn create_scaled_image(img: DynamicImage, size: u32, pad_to_square: bool) -> Result<Vec<u8>> {
    if pad_to_square {
        // Create a square canvas with a white background
        let mut canvas = image::RgbImage::from_fn(size, size, |_, _| {
            image::Rgb([255, 255, 255]) // White background
        });

        // Scale the image with aspect ratio preservation
        let scaled = img.resize(size, size, image::imageops::FilterType::Lanczos3);

        // Get dimensions and calculate position for centering
        let (width, height) = scaled.dimensions();
        let x_offset = (size - width) / 2;
        let y_offset = (size - height) / 2;

        // Copy the scaled image to the center
        image::imageops::overlay(
            &mut canvas,
            &scaled.to_rgb8(),
            x_offset as i64,
            y_offset as i64,
        );

        // Encode to JPEG using turbojpeg
        let jpeg_data = turbojpeg::compress_image(&canvas, 85, turbojpeg::Subsamp::None)
            .with_context(|| "Failed to compress image with turbojpeg")?;

        Ok(jpeg_data.to_vec())
    } else {
        // Just resize the image to the given size (max dimension) while maintaining the aspect ratio
        let scaled = img.resize(size, size, image::imageops::FilterType::Lanczos3);
        let mut jpeg_data = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_data, 85);
        encoder.encode(
            scaled.as_bytes(),
            scaled.width(),
            scaled.height(),
            scaled.color().into(),
        )?;
        Ok(jpeg_data)
    }
}

pub fn create_scaled_image_in_memory(source_path: &Path, image_type: ImageType) -> Result<Vec<u8>> {
    let size = image_type.size();
    let pad_to_square = image_type.pad_to_square();

    let mut img = image::open(source_path)
        .with_context(|| format!("Failed to open image: {:?}", source_path))?;

    // Apply EXIF orientation
    img = crate::exif_parser::apply_exif_orientation(source_path, img)?;

    create_scaled_image(img, size, pad_to_square)
}

/// Image types for processing
#[derive(Debug, Clone, Copy)]
pub enum ImageType {
    Marker,
    Thumbnail,
    Popup,
}

impl ImageType {
    /// Returns the size of the image in pixels
    pub fn size(&self) -> u32 {
        match self {
            ImageType::Marker => MARKER_SIZE,
            ImageType::Thumbnail => THUMBNAIL_SIZE,
            ImageType::Popup => POPUP_SIZE,
        }
    }

    /// Returns a human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            ImageType::Marker => "marker",
            ImageType::Thumbnail => "thumbnail",
            ImageType::Popup => "popup",
        }
    }

    /// Returns whether the image should be padded to a square
    pub fn pad_to_square(&self) -> bool {
        match self {
            ImageType::Marker | ImageType::Thumbnail => true,
            ImageType::Popup => false,
        }
    }
}

/// Конвертирует HEIC файл в JPEG с указанными размерами, используя нативный код
fn convert_heic_to_jpeg_native(photo: &PhotoMetadata, size_param: &str) -> Result<Vec<u8>> {
    let max_dimension = match size_param {
        "thumbnail" => THUMBNAIL_SIZE,
        "marker" => MARKER_SIZE,
        "popup" => POPUP_SIZE,
        _ => 4096, // A reasonable default for 'full size'
    };

    let pad_to_square = match size_param {
        "thumbnail" | "marker" => true,
        _ => false,
    };

    let original_path = Path::new(&photo.file_path);
    let mut path_to_decode = original_path.to_path_buf();
    let mut temp_symlink_path: Option<PathBuf> = None;

    // Check the file extension
    let ext_lower = original_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // If it's HEIC/HEIF and the extension is not lowercase, create a temporary symlink
    if (ext_lower == "heic" || ext_lower == "heif")
        && original_path
            .extension()
            .map_or(false, |ext| ext.to_ascii_lowercase() != ext)
    {
        let parent = original_path.parent().unwrap_or_else(|| Path::new("."));
        let filename_stem = original_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("temp_heic");

        // Create a unique name for the symlink to avoid collisions
        let mut counter = 0;
        let final_symlink_path = loop {
            let current_symlink_name = format!("{}_{}.{}", filename_stem, counter, ext_lower);
            let current_symlink_path = parent.join(current_symlink_name);
            if !current_symlink_path.exists() {
                break current_symlink_path;
            }
            counter += 1;
        };

        std::os::unix::fs::symlink(original_path, &final_symlink_path).with_context(|| {
            format!(
                "Failed to create symlink for HEIC file: {:?}",
                original_path
            )
        })?;
        path_to_decode = final_symlink_path.clone();
        temp_symlink_path = Some(final_symlink_path);
    }

    let img = ImageReader::open(&path_to_decode)?
        .with_guessed_format()?
        .decode()
        .with_context(|| format!("Failed to decode image: {:?}", &path_to_decode))?;

    // Remove the temporary symlink if it was created
    if let Some(symlink) = temp_symlink_path {
        let _ = std::fs::remove_file(&symlink);
    }

    create_scaled_image(img, max_dimension, pad_to_square)
}

/// Converts a HEIC file to JPEG with the specified dimensions
pub fn convert_heic_to_jpeg(photo: &PhotoMetadata, size_param: &str) -> Result<Vec<u8>> {
    // First, try the native method
    if let Ok(data) = convert_heic_to_jpeg_native(photo, size_param) {
        return Ok(data);
    }

    // As a fallback on macOS, use sips
    if cfg!(target_os = "macos") {
        if let Ok(output) = std::process::Command::new("sips")
            .arg("-s")
            .arg("format")
            .arg("jpeg")
            .arg(&photo.file_path)
            .arg("--out")
            .arg("-")
            .output()
        {
            if output.status.success() {
                return Ok(output.stdout);
            }
        }
    }

    anyhow::bail!("Failed to convert HEIC file: {}", photo.file_path)
}
