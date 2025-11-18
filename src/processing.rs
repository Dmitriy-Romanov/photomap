use anyhow::Result;
use ignore::Walk;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use crate::database::{Database, PhotoMetadata};
use crate::exif_parser::{extract_metadata_from_heic, extract_metadata_from_jpeg, get_gps_coord, get_datetime_from_exif};

/// Processes photos and saves metadata to the database
/// Returns processing statistics: (total_files, processed_count, gps_count, no_gps_count, heic_count)
pub fn process_photos_with_stats(db: &Database, photos_dir: &Path, silent_mode: bool, clear_database: bool) -> Result<(usize, usize, usize, usize, usize)> {
    if !silent_mode {
        println!("🔍 Scanning photos directory: {}", photos_dir.display());
    }

    if !photos_dir.exists() {
        let error_msg = format!("❌ Photos directory not found: {}", photos_dir.display());
        if silent_mode {
            return Err(anyhow::Error::msg(error_msg));
        } else {
            println!("{}", error_msg);
            return Ok((0, 0, 0, 0, 0));
        }
    }

    // Clear existing photos from database before processing new folder
    if clear_database {
        if !silent_mode {
            println!("🗑️  Clearing existing photos from database...");
        }
        db.clear_all_photos()?;
        if !silent_mode {
            println!("✅ Database cleared successfully");
        }
    }

    // Create walker for photos directory only
    let walker = Walk::new(photos_dir);
    let files: Vec<PathBuf> = walker
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|e| {
            // Check that file is in photos directory
            e.path().starts_with(photos_dir)
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

    let total_files = files.len();
    if !silent_mode {
        println!("✅ Found {} files in photos directory. Starting processing...", total_files);
    }

    // Count HEIC files
    let heic_count = files.iter()
        .filter(|path| {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                matches!(ext.to_lowercase().as_str(), "heic" | "heif")
            } else {
                false
            }
        })
        .count();

    // Process files in parallel using Rayon with timing
    let start_time = std::time::Instant::now();

    if !silent_mode {
        println!("📊 Processing {} files with parallel optimization...", total_files);
    }

    // Process files in parallel and count successes
    let processed_photos: Vec<_> = files
        .par_iter()
        .map(|path| {
            let result = process_file_to_database(path, db, photos_dir);
            result
        })
        .collect();

    // Count successful results by checking each result
    let successful_count = processed_photos.iter().filter(|r| r.is_ok()).count();

    let processing_time = start_time.elapsed();
    let processing_secs = processing_time.as_secs_f64();
    let avg_time_per_file_ms = if total_files > 0 {
        (processing_secs * 1000.0) / total_files as f64
    } else {
        0.0
    };

    let final_count = successful_count;
    let gps_count = final_count; // All successfully processed have GPS data
    let no_gps_count = total_files - final_count;

    // Print processing statistics
    if !silent_mode {
        println!("\n📊 Статистика обработки:");
        println!("   🔍 Всего файлов проверено: {}", total_files);
        println!("   📸 Обработано фотографий: {}", final_count);
        println!("   🗺️  С GPS-данными: {}", gps_count);
        println!("   ❌ Без GPS: {}", no_gps_count);
        println!("   📱 HEIC файлов: {}", heic_count);
        println!("   📷 JPEG/другие: {}", if final_count >= heic_count { final_count - heic_count } else { 0 });
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
    }

    Ok((total_files, final_count, gps_count, no_gps_count, heic_count))
}

/// Simplified version of the function for backward compatibility
pub fn process_photos_into_database(db: &Database, photos_dir: &Path) -> Result<()> {
    process_photos_with_stats(db, photos_dir, true, true)?;
    Ok(())
}

/// Processes photos from the specified folder and sends progress events
pub fn process_photos_from_directory(db: &Database, photos_dir: &Path) -> Result<(usize, usize, usize, usize, usize)> {
    println!("🔍 Processing photos from directory: {}", photos_dir.display());

    // Use the new combined function, but without silent_mode
    process_photos_with_stats(db, photos_dir, false, true)
}

/// Processes a single file and saves it to the database
fn process_file_to_database(path: &Path, db: &Database, photos_dir: &Path) -> Result<()> {
    // Check the file extension, saving it in lowercase for checks
    let ext_lower = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Basic list of supported formats
    let supported_formats = ["jpg", "jpeg", "png", "tiff", "tif", "webp", "bmp", "gif", "heic", "heif", "avif"];

    if !supported_formats.contains(&ext_lower.as_str()) {
        anyhow::bail!("File is not a supported image");
    }

    // Check if it's HEIC or not, using the lowercase version
    let is_heif = matches!(ext_lower.as_str(), "heic" | "heif" | "avif");

    // --- GPS and date extraction ---
    let (lat, lng, datetime) = if is_heif {
        // Try to extract metadata from HEIC
        match extract_metadata_from_heic(path) {
            Ok(data) => data,
            Err(e) => {
                anyhow::bail!("HEIC GPS data not found: {}", e);
            }
        }
    } else {
        // For standard formats, use our parsers
        if ext_lower == "jpg" || ext_lower == "jpeg" {
            // Use our own JPEG parser
            match extract_metadata_from_jpeg(path) {
                Ok(data) => data,
                Err(e) => {
                    anyhow::bail!("JPEG GPS data not found: {}", e);
                }
            }
        } else {
            // For other formats (PNG, TIFF, etc.), keep the old method
            let file = fs::File::open(path)?;
            let mut bufreader = std::io::BufReader::new(&file);
            let exifreader = exif::Reader::new();
            let exif = exifreader.read_from_container(&mut bufreader)?;

            let lat = get_gps_coord(&exif, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef)?;
            let lng = get_gps_coord(&exif, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef)?;

            if lat.is_none() || lng.is_none() {
                anyhow::bail!("GPS data not found");
            }

            let datetime = get_datetime_from_exif(&exif).unwrap_or_else(|| "Дата неизвестна".to_string());

            (lat.unwrap(), lng.unwrap(), datetime)
        }
    };

    // --- Create a database record ---
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::Error::msg("Invalid file name"))?;

    // Generate relative path from photos directory
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

    // Save to the database
    db.insert_photo(&photo_metadata)?;

    Ok(())
}
