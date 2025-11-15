use anyhow::{Context, Result};
use ignore::Walk;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;

// Import modules
mod database;
mod image_processing;
mod exif_parser;
mod html_template;
mod server;

use database::{Database, PhotoMetadata};
use image_processing::check_imagemagick;
use server::{AppState, start_server};

// Global HEIC support flag
static mut HAS_IMAGEMAGICK: bool = false;
static mut HEIC_SUPPORTED: bool = false;

/// Обрабатывает фотографии и сохраняет метаданные в базу данных
fn process_photos_into_database(db: &Database) -> Result<()> {
    println!("🔍 Scanning photos directory...");
    let photos_dir = Path::new("/Users/dmitriiromanov/claude/photomap/photos");
    if !photos_dir.exists() {
        println!("❌ Photos directory not found: {}", photos_dir.display());
        return Ok(());
    }
    println!("📂 Photos directory: {}", photos_dir.display());

    // Create walker for photos directory only
    let walker = Walk::new(photos_dir);
    let files: Vec<PathBuf> = walker
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|e| {
            // Check that file is in photos directory
            e.path().starts_with(&photos_dir)
        })
        .filter(|e| {
            // Exclude system directories and hidden files
            let path = e.path();
            if let Some(components) = path.components().collect::<Vec<_>>().get(1..) {
                for component in components {
                    if let Some(name) = component.as_os_str().to_str() {
                        if name.starts_with('.') || name == "node_modules" || name == "target" || name == ".git" {
                            return false;
                        }
                    }
                }
            }
            true
        })
        .filter(|e| e.file_type().map_or(false, |ft| ft.is_file()))
        .map(|e| e.into_path())
        .collect();
    println!("✅ Found {} files in photos directory. Starting processing...", files.len());

    // Process files in parallel using Rayon with timing
    let start_time = std::time::Instant::now();
    let total_files = files.len();

    println!("📊 Processing {} files with parallel optimization...", total_files);

    let processed_count = Arc::new(Mutex::new(0usize));
    let heic_count = files.iter()
        .filter(|path| {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                matches!(ext.to_lowercase().as_str(), "heic" | "heif")
            } else {
                false
            }
        })
        .count();

    let processed_photos: Vec<_> = files
        .par_iter()
        .map(|path| {
            let result = process_file_to_database(path, db);
            match &result {
                Ok(_) => {
                    if let Ok(mut count) = processed_count.lock() {
                        *count += 1;
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Warning: Failed to process {}: {}", path.display(), e);
                }
            }
            result
        })
        .collect();

    // Count successful results
    let successful_count = processed_photos.iter().filter(|r| r.is_ok()).count();

    let processing_time = start_time.elapsed();
    let processing_secs = processing_time.as_secs_f64();
    let avg_time_per_file_ms = if total_files > 0 {
        (processing_secs * 1000.0) / total_files as f64
    } else {
        0.0
    };

    let final_count = successful_count;

    // Print processing statistics
    println!("\n📊 Статистика обработки:");
    println!("   🔍 Всего файлов проверено: {}", total_files);
    println!("   📸 Обработано фотографий: {}", final_count);
    println!("   🗺️  С GPS-данными: {}", final_count);
    println!("   ❌ Без GPS: {}", total_files - final_count);
    println!("   📱 HEIC файлов: {}", heic_count);
    println!("   📷 JPEG/другие: {}", final_count - heic_count);
    println!("   ⏱️  Время обработки: {:.2} сек", processing_secs);
    println!("   📈 Среднее время на файл: {:.1} мс", avg_time_per_file_ms);

    // Performance prediction for large collections
    if total_files >= 100 {
        let predicted_10k_time = (avg_time_per_file_ms * 10000.0) / 1000.0;
        let predicted_100k_time = (avg_time_per_file_ms * 100000.0) / 1000.0;

        println!("\n🔮 Прогноз производительности:");
        println!("   📊 Для 10,000 фото: ~{:.1} минут", predicted_10k_time / 60.0);
        println!("   📊 Для 100,000 фото: ~{:.1} минут", predicted_100k_time / 60.0);
        println!("   💡 On-demand генерация маркеров: ~0% времени на старте!");
        println!("   💡 Экономия диска: {} файлов не создается", total_files * 2); // ~2KB per saved thumbnail
    }

    println!("\n🎉 Обработка завершена! Данные сохранены в базу данных 'photomap.db'.");
    println!("   🗄️  База данных содержит {} фотографий с GPS-данными", final_count);

    Ok(())
}

/// Обрабатывает один файл и сохраняет в базу данных
fn process_file_to_database(path: &Path, db: &Database) -> Result<()> {
    // Проверяем расширение файла
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase());

    // Базовый список поддерживаемых форматов
    let supported_formats = ["jpg", "jpeg", "png", "tiff", "tif", "webp", "bmp", "gif", "heic", "heif", "avif"];

    if !supported_formats.contains(&ext.as_deref().unwrap_or("")) {
        anyhow::bail!("Файл не является поддерживаемым изображением");
    }

    // Проверяем, это HEIC или нет
    let is_heif = matches!(ext.as_deref(), Some("heic") | Some("heif") | Some("avif"));

    // Пропускаем HEIC файлы если ImageMagick не доступен
    if is_heif {
        unsafe {
            if !HEIC_SUPPORTED {
                anyhow::bail!("HEIC файл пропущен - ImageMagick не установлен");
            }
        }
    }

    // --- Извлечение GPS и даты ---
    let (lat, lng, datetime) = if is_heif {
        // Пытаемся извлечь метаданные из HEIC
        match exif_parser::extract_metadata_from_heif_custom(path) {
            Ok(data) => data,
            Err(e) => {
                anyhow::bail!("HEIC GPS данные не найдены: {}", e);
            }
        }
    } else {
        // Для стандартных форматов используем наши парсеры
        let ext = path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        if ext == "jpg" || ext == "jpeg" {
            // Используем наш собственный JPEG парсер
            match exif_parser::extract_metadata_from_jpeg_custom(path) {
                Ok(data) => data,
                Err(e) => {
                    anyhow::bail!("JPEG GPS данные не найдены: {}", e);
                }
            }
        } else {
            // Для остальных форматов (PNG, TIFF и т.д.) оставляем старый метод
            let file = fs::File::open(path)?;
            let mut bufreader = std::io::BufReader::new(&file);
            let exifreader = exif::Reader::new();
            let exif = exifreader.read_from_container(&mut bufreader)?;

            let lat = exif_parser::get_gps_coord(&exif, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef)?;
            let lng = exif_parser::get_gps_coord(&exif, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef)?;

            if lat.is_none() || lng.is_none() {
                anyhow::bail!("GPS-данные не найдены");
            }

            let datetime = exif_parser::get_datetime_from_exif(&exif).unwrap_or_else(|| "Дата неизвестна".to_string());

            (lat.unwrap(), lng.unwrap(), datetime)
        }
    };

    // --- Создание записи в базе данных ---
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::Error::msg("Некорректное имя файла"))?;

    // Generate relative path from photos directory
    let photos_dir = Path::new("/Users/dmitriiromanov/claude/photomap/photos");
    let relative_path = path
        .strip_prefix(photos_dir)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| filename.to_string());

    let photo_metadata = PhotoMetadata {
        filename: filename.to_string(),
        relative_path,
        datetime,
        lat,
        lng,
        file_path: path.to_string_lossy().to_string(),
        is_heic: is_heif,
    };

    // Сохраняем в базу данных
    db.insert_photo(&photo_metadata)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🗺️  PhotoMap Processor v3.0 - SQLite + On-demand markers starting...");

    // Check ImageMagick availability for HEIC support
    let has_imagemagick = check_imagemagick();
    unsafe {
        HAS_IMAGEMAGICK = has_imagemagick;
        HEIC_SUPPORTED = has_imagemagick;
    }

    if has_imagemagick {
        println!("✅ ImageMagick detected - HEIC files supported");
    } else {
        println!("⚠️  ImageMagick not found - HEIC files will be skipped");
        println!("   Install ImageMagick to enable HEIC processing: brew install imagemagick");
    }

    // Initialize database
    println!("🗄️  Initializing database...");
    let db = Database::new()
        .with_context(|| "Failed to initialize database")?;
    println!("✅ Database initialized successfully");

    // Process photos
    process_photos_into_database(&db)?;

    println!("\n🎉 Phase 3 implementation ready!");
    println!("   📊 {} photos with GPS data in database", db.get_photos_count()?);
    println!("   🚀 Starting HTTP server for on-demand marker generation");

    // Start HTTP server
    let app_state = AppState {
        db,
        has_heic_support: has_imagemagick,
    };

    start_server(app_state).await?;

    Ok(())
}