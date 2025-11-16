use anyhow::{Context, Result};
use std::io::Cursor;
use std::path::Path;
use std::process::Command;
use image::GenericImageView;
use crate::database::PhotoMetadata;
use crate::constants::*;

/// Создает маленькую иконку маркера для изображения (40x40px PNG с прозрачностью и центрированием) в памяти.
pub fn create_marker_icon_in_memory(source_path: &Path) -> Result<Vec<u8>> {
    let mut img = image::open(source_path)
        .with_context(|| format!("Не удалось открыть изображение: {:?}", source_path))?;

    // Применяем EXIF-ориентацию
    img = crate::exif_parser::apply_exif_orientation(source_path, img)?;

    // Создаем квадратное изображение с ПРОЗРАЧНЫМ фоном
    let mut canvas = image::RgbaImage::from_fn(MARKER_SIZE, MARKER_SIZE, |_, _| {
        image::Rgba([0, 0, 0, 0]) // Полностью прозрачный фон
    });

    // Масштабируем изображение с сохранением пропорций
    let scaled = img.resize(MARKER_SIZE, MARKER_SIZE, image::imageops::FilterType::Lanczos3);

    // Получаем размеры и вычисляем позицию для центрирования
    let (width, height) = scaled.dimensions();
    let x_offset = (MARKER_SIZE - width as u32) / 2;
    let y_offset = (MARKER_SIZE - height as u32) / 2;

    // Копируем масштабированное изображение в центр
    image::imageops::overlay(&mut canvas, &scaled.to_rgba8(), x_offset as i64, y_offset as i64);

    // Конвертируем в PNG в память
    let final_img = image::DynamicImage::ImageRgba8(canvas);
    let mut buffer = Vec::new();
    {
        let mut cursor = Cursor::new(&mut buffer);
        final_img.write_to(&mut cursor, image::ImageFormat::Png)?;
    }

    Ok(buffer)
}

/// Создает миниатюру большего размера для отображения на маркерах (60x60px) в памяти.
pub fn create_thumbnail_in_memory(source_path: &Path) -> Result<Vec<u8>> {
    let mut img = image::open(source_path)
        .with_context(|| format!("Не удалось открыть изображение: {:?}", source_path))?;

    // Применяем EXIF-ориентацию
    img = crate::exif_parser::apply_exif_orientation(source_path, img)?;

    // Создаем квадратное изображение с ПРОЗРАЧНЫМ фоном
    let mut canvas = image::RgbaImage::from_fn(THUMBNAIL_SIZE, THUMBNAIL_SIZE, |_, _| {
        image::Rgba([0, 0, 0, 0]) // Полностью прозрачный фон
    });

    // Масштабируем изображение с сохранением пропорций
    let scaled = img.resize(THUMBNAIL_SIZE, THUMBNAIL_SIZE, image::imageops::FilterType::Lanczos3);

    // Получаем размеры и вычисляем позицию для центрирования
    let (width, height) = scaled.dimensions();
    let x_offset = (THUMBNAIL_SIZE - width as u32) / 2;
    let y_offset = (THUMBNAIL_SIZE - height as u32) / 2;

    // Копируем масштабированное изображение в центр
    image::imageops::overlay(&mut canvas, &scaled.to_rgba8(), x_offset as i64, y_offset as i64);

    // Конвертируем в PNG в память
    let final_img = image::DynamicImage::ImageRgba8(canvas);
    let mut buffer = Vec::new();
    {
        let mut cursor = Cursor::new(&mut buffer);
        final_img.write_to(&mut cursor, image::ImageFormat::Png)?;
    }

    Ok(buffer)
}

/// Конвертирует HEIC файл в JPEG с указанными размерами
pub fn convert_heic_to_jpeg(photo: &PhotoMetadata, size_param: &str) -> Result<Vec<u8>> {
    // Determine ImageMagick parameters based on size request
    let thumbnail_extent = format!("{}x{}", THUMBNAIL_SIZE, THUMBNAIL_SIZE);
    let marker_extent = format!("{}x{}", MARKER_SIZE, MARKER_SIZE);

    let magick_args = match size_param {
        "thumbnail" => {
            vec![
                &photo.file_path,
                "-resize", THUMBNAIL_RESIZE_PARAMS,   // Only resize if larger, preserve aspect ratio
                "-gravity", "center",
                "-extent", thumbnail_extent.as_str(),    // Pad to exact square with transparent background
                "-quality", "80",
                "jpg:-"
            ]
        }
        "marker" => {
            vec![
                &photo.file_path,
                "-resize", MARKER_RESIZE_PARAMS,   // Only resize if larger, preserve aspect ratio
                "-gravity", "center",
                "-extent", marker_extent.as_str(),    // Pad to exact square with transparent background
                "-quality", "80",
                "jpg:-"
            ]
        }
        _ => {
            // Full size
            vec![
                &photo.file_path,
                "jpg:-"
            ]
        }
    };

    // Use ImageMagick if available, otherwise return error
    let mut cmd = std::process::Command::new("magick");
    for arg in magick_args {
        cmd.arg(arg);
    }

    if let Ok(output) = cmd.output() {
        if output.status.success() {
            return Ok(output.stdout);
        }
    }

    // Try sips on macOS
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

    anyhow::bail!("Failed to convert HEIC file using ImageMagick or sips")
}

/// Проверяет доступность ImageMagick для обработки HEIC
pub fn check_imagemagick() -> bool {
    Command::new("magick")
        .arg("--version")
        .output()
        .map(|_| true)
        .or_else(|_| {
            Command::new("convert")
                .arg("-version")
                .output()
                .map(|_| true)
        })
        .unwrap_or(false)
}