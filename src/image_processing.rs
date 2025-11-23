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
        // Use Triangle filter for faster resizing (sufficient for thumbnails)
        let scaled = img.resize(size, size, image::imageops::FilterType::Triangle);

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
        let scaled = img.resize(size, size, image::imageops::FilterType::Triangle);
        
        // Convert to RGB8 and encode with turbojpeg (faster than image crate's encoder)
        let rgb_image = scaled.to_rgb8();
        let jpeg_data = turbojpeg::compress_image(&rgb_image, 85, turbojpeg::Subsamp::None)
            .with_context(|| "Failed to compress image with turbojpeg")?;
        
        Ok(jpeg_data.to_vec())
    }
}

fn try_load_jpeg(path: &Path, target_size: u32) -> Result<Option<DynamicImage>> {
    let data = std::fs::read(path)?;
    
    // Check for JPEG magic bytes (FF D8 FF)
    if data.len() < 3 || data[0] != 0xFF || data[1] != 0xD8 || data[2] != 0xFF {
        return Ok(None);
    }

    // Try to decompress with turbojpeg (much faster than image crate)
    let mut decompressor = turbojpeg::Decompressor::new()?;
    let header = decompressor.read_header(&data)?;
    
    // Calculate the best scaling factor
    let scaling_factor = if target_size > 0 {
        let _min_dim = std::cmp::min(header.width, header.height);
        let factors = turbojpeg::Decompressor::supported_scaling_factors();
        
        // Find the smallest factor that produces an image >= target_size
        factors.iter()
            .filter(|f| {
                let scaled_w = (header.width * f.num()).div_ceil(f.denom());
                let scaled_h = (header.height * f.num()).div_ceil(f.denom());
                let scaled_min = std::cmp::min(scaled_w, scaled_h);
                scaled_min >= target_size as usize
            })
            .min_by_key(|f| {
                // We prefer the smallest sufficient factor (closest to target)
                // Since they are fractions, we can compare their float value or just use the one found
                // Actually, we want the *smallest* factor that is *sufficient*.
                // Factors are usually 1/8, 1/4, 3/8, 1/2, ... 1/1.
                // Smaller factor = smaller image.
                // So we want the minimum factor that satisfies the condition.
                (f.num() * 100) / f.denom()
            })
            .cloned()
            .unwrap_or(turbojpeg::ScalingFactor::new(1, 1))
    } else {
        turbojpeg::ScalingFactor::new(1, 1)
    };

    decompressor.set_scaling_factor(scaling_factor)?;
    
    // Decompress directly into an ImageBuffer
    // Note: decompress_image creates the buffer for us, but it doesn't seem to expose scaling easily?
    // Wait, if I use `decompressor.decompress`, I need to provide the buffer.
    // Let's try to use `decompressor.decompress` with a manually created buffer.
    
    // Re-read header to get scaled dimensions? Or calculate them?
    // The API might update header info or we need to calculate.
    // Let's assume we need to calculate or use `decompressor` to get output info.
    // Actually, `turbojpeg-rs` documentation says `read_header` returns `Header`.
    // `ScalingFactor` has `apply_to(width, height)`.
    
    let scaled_width = (header.width * scaling_factor.num()).div_ceil(scaling_factor.denom());
    let scaled_height = (header.height * scaling_factor.num()).div_ceil(scaling_factor.denom());
    
    let mut image = image::RgbImage::new(scaled_width as u32, scaled_height as u32);
    
    // We need to wrap the buffer in turbojpeg::Image
    // image::RgbImage is a flat buffer of RGB pixels
    let turbo_image = turbojpeg::Image {
        pixels: image.as_mut(),
        width: scaled_width,
        height: scaled_height,
        format: turbojpeg::PixelFormat::RGB,
        pitch: scaled_width * 3,
    };
    
    match decompressor.decompress(&data, turbo_image) {
        Ok(_) => Ok(Some(DynamicImage::ImageRgb8(image))),
        Err(_) => Ok(None),
    }
}

pub fn create_scaled_image_in_memory(source_path: &Path, image_type: ImageType) -> Result<Vec<u8>> {
    let size = image_type.size();
    let pad_to_square = image_type.pad_to_square();

    // Try to load with turbojpeg first (fast path for JPEGs)
    // We pass target_size to allow for future optimization with scaling
    let mut img = if let Ok(Some(img)) = try_load_jpeg(source_path, size) {
        img
    } else {
        image::open(source_path)
            .with_context(|| format!("Failed to open image: {:?}", source_path))?
    };

    // Apply EXIF orientation
    img = crate::exif_parser::apply_exif_orientation(source_path, img)?;

    create_scaled_image(img, size, pad_to_square)
}

/// Image types for processing
#[derive(Debug, Clone, Copy)]
pub enum ImageType {
    Marker,
    Thumbnail,
    Gallery,
    Popup,
}

impl ImageType {
    /// Returns the size of the image in pixels
    pub fn size(&self) -> u32 {
        match self {
            ImageType::Marker => MARKER_SIZE,
            ImageType::Thumbnail => THUMBNAIL_SIZE,
            ImageType::Gallery => GALLERY_SIZE,
            ImageType::Popup => POPUP_SIZE,
        }
    }

    /// Returns a human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            ImageType::Marker => "marker",
            ImageType::Thumbnail => "thumbnail",
            ImageType::Gallery => "gallery",
            ImageType::Popup => "popup",
        }
    }

    /// Returns whether the image should be padded to a square
    pub fn pad_to_square(&self) -> bool {
        match self {
            ImageType::Marker | ImageType::Thumbnail | ImageType::Gallery => true,
            ImageType::Popup => false,
        }
    }
}

/// Converts a HEIC file to JPEG with specified dimensions using native code
fn convert_heic_to_jpeg_native(photo: &PhotoMetadata, size_param: &str) -> Result<Vec<u8>> {
    let max_dimension = match size_param {
        "marker" => MARKER_SIZE,
        "thumbnail" => THUMBNAIL_SIZE,
        "gallery" => GALLERY_SIZE,
        "popup" => POPUP_SIZE,
        _ => 4096, // A reasonable default for 'full size'
    };

    let pad_to_square = matches!(size_param, "marker" | "thumbnail" | "gallery");

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
            .is_some_and(|ext| ext.to_ascii_lowercase() != ext)
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

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(original_path, &final_symlink_path).with_context(|| {
                format!(
                    "Failed to create symlink for HEIC file: {:?}",
                    original_path
                )
            })?;
        }

        #[cfg(not(unix))]
        {
            // On Windows and other non-Unix systems, we copy the file instead of symlinking
            // because symlinks require special privileges on Windows
            std::fs::copy(original_path, &final_symlink_path).with_context(|| {
                format!(
                    "Failed to copy HEIC file for decoding: {:?}",
                    original_path
                )
            })?;
        }
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
