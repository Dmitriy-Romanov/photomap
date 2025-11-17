use anyhow::{Context, Result};

use std::path::{Path, PathBuf};
use image::{DynamicImage, GenericImageView, ImageReader};
use crate::database::PhotoMetadata;
use crate::constants::*;

/// Создает квадратную JPG миниатюру с белым фоном из уже загруженного изображения
fn create_padded_thumbnail(img: DynamicImage, size: u32) -> Result<Vec<u8>> {
    // Создаем квадратное изображение с БЕЛЫМ фоном
    let mut canvas = image::RgbImage::from_fn(size, size, |_, _| {
        image::Rgb([255, 255, 255]) // Белый фон
    });

    // Масштабируем изображение с сохранением пропорций
    let scaled = img.resize(size, size, image::imageops::FilterType::Lanczos3);

    // Получаем размеры и вычисляем позицию для центрирования
    let (width, height) = scaled.dimensions();
    let x_offset = (size - width) / 2;
    let y_offset = (size - height) / 2;

    // Копируем масштабированное изображение в центр
    image::imageops::overlay(&mut canvas, &scaled.to_rgb8(), x_offset as i64, y_offset as i64);

    // Кодируем в JPEG с помощью turbojpeg
    let jpeg_data = turbojpeg::compress_image(&canvas, 85, turbojpeg::Subsamp::None)
        .with_context(|| "Не удалось сжать изображение с помощью turbojpeg")?;

    Ok(jpeg_data.to_vec())
}

pub fn create_scaled_image_in_memory(source_path: &Path, image_type: ImageType) -> Result<Vec<u8>> {
    let size = image_type.size();

    let mut img = image::open(source_path)
        .with_context(|| format!("Не удалось открыть изображение: {:?}", source_path))?;

    // Применяем EXIF-ориентацию
    img = crate::exif_parser::apply_exif_orientation(source_path, img)?;

    create_padded_thumbnail(img, size)
}

/// Типы изображений для обработки
#[derive(Debug, Clone, Copy)]
pub enum ImageType {
    Marker,
    Thumbnail,
}

impl ImageType {
    /// Возвращает размер изображения в пикселях
    pub fn size(&self) -> u32 {
        match self {
            ImageType::Marker => MARKER_SIZE,
            ImageType::Thumbnail => THUMBNAIL_SIZE,
        }
    }

    /// Возвращает человекочитаемое название
    pub fn name(&self) -> &'static str {
        match self {
            ImageType::Marker => "marker",
            ImageType::Thumbnail => "thumbnail",
        }
    }
}

/// Конвертирует HEIC файл в JPEG с указанными размерами, используя нативный код
fn convert_heic_to_jpeg_native(photo: &PhotoMetadata, size_param: &str) -> Result<Vec<u8>> {
    let max_dimension = match size_param {
        "thumbnail" => THUMBNAIL_SIZE,
        "marker" => MARKER_SIZE,
        _ => 4096, // A reasonable default for 'full size'
    };

    let original_path = Path::new(&photo.file_path);
    let mut path_to_decode = original_path.to_path_buf();
    let mut temp_symlink_path: Option<PathBuf> = None;

    // Проверяем расширение файла
    let ext_lower = original_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Если это HEIC/HEIF и расширение не в нижнем регистре, создаем временную симлинк
    if (ext_lower == "heic" || ext_lower == "heif") && original_path.extension().map_or(false, |ext| ext.to_ascii_lowercase() != ext) {
        let parent = original_path.parent().unwrap_or_else(|| Path::new("."));
        let filename_stem = original_path.file_stem().and_then(|s| s.to_str()).unwrap_or("temp_heic");



        // Создаем уникальное имя для симлинка, чтобы избежать коллизий
        let mut counter = 0;
        let final_symlink_path = loop {
            let current_symlink_name = format!("{}_{}.{}", filename_stem, counter, ext_lower);
            let current_symlink_path = parent.join(current_symlink_name);
            if !current_symlink_path.exists() {
                break current_symlink_path;
            }
            counter += 1;
        };

        std::os::unix::fs::symlink(original_path, &final_symlink_path)
            .with_context(|| format!("Не удалось создать симлинк для HEIC файла: {:?}", original_path))?;
        path_to_decode = final_symlink_path.clone();
        temp_symlink_path = Some(final_symlink_path);
    }

    let img = ImageReader::open(&path_to_decode)?
        .with_guessed_format()?
        .decode()
        .with_context(|| format!("Не удалось декодировать изображение: {:?}", &path_to_decode))?;

    // Удаляем временную симлинк, если она была создана
    if let Some(symlink) = temp_symlink_path {
        let _ = std::fs::remove_file(&symlink);
    }

    create_padded_thumbnail(img, max_dimension)
}

/// Конвертирует HEIC файл в JPEG с указанными размерами
pub fn convert_heic_to_jpeg(photo: &PhotoMetadata, size_param: &str) -> Result<Vec<u8>> {
    // Сначала пробуем нативный метод
    if let Ok(data) = convert_heic_to_jpeg_native(photo, size_param) {
        return Ok(data);
    }

    // В качестве запасного варианта на macOS используем sips
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

    anyhow::bail!("Не удалось конвертировать HEIC файл: {}", photo.file_path)
}