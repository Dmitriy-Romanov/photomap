use crate::database::{Database, PhotoMetadata};
use crate::exif_parser::{
    extract_metadata_from_heic, extract_metadata_from_jpeg, get_datetime_from_exif, get_gps_coord,
};
use anyhow::Result;
use ignore::Walk;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use tracing::{error, info, warn};

/// Processes photos and saves metadata to the database
/// Returns processing statistics: (total_files, processed_count, gps_count, no_gps_count, heic_count)
pub fn process_photos_with_stats(
    db: &Database,
    photos_dir: &Path,
    silent_mode: bool,
    clear_database: bool,
) -> Result<(usize, usize, usize, usize, usize)> {
    if !silent_mode {
        info!("🔍 Scanning photos directory: {}", photos_dir.display());
    }

    if !photos_dir.exists() {
        let error_msg = format!("❌ Photos directory not found: {}", photos_dir.display());
        if silent_mode {
            return Err(anyhow::Error::msg(error_msg));
        } else {
            error!("{}", error_msg);
            return Ok((0, 0, 0, 0, 0));
        }
    }

    // Clear existing photos from database before processing new folder
    if clear_database {
        if !silent_mode {
            info!("🗑️  Clearing existing photos from database...");
        }
        db.clear_all_photos()?;
        if !silent_mode {
            info!("✅ Database cleared successfully");
        }
    }

    // Create walker for photos directory only
    let walker = Walk::new(photos_dir);

    // Process files in parallel using Rayon with timing
    let start_time = std::time::Instant::now();

    if !silent_mode {
        info!("📊 Starting parallel processing of files...");
    }

    let reduction_result = walker
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
                        if name.starts_with('.')
                            || name == "node_modules"
                            || name == "target"
                            || name == ".git"
                        {
                            return false;
                        }
                    }
                }
            }
            true
        })
        .filter(|e| {
            // Filter by extension - only process supported image formats
            // This prevents trying to process video files or other non-images
            if let Some(ext) = e.path().extension().and_then(|s| s.to_str()) {
                let ext_lower = ext.to_lowercase();
                matches!(
                    ext_lower.as_str(),
                    "jpg" | "jpeg" | "heic" | "heif" | "avif"
                )
            } else {
                false
            }
        })
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .par_bridge() // Use par_bridge to enable parallel processing on the iterator
        .fold(
            || (vec![], 0usize, 0usize), // Initial state for each thread: (photo_metadata_vec, total_files, heic_count)
            |mut acc, entry| {
                let path = entry.into_path();
                acc.1 += 1; // Increment total_files

                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if matches!(ext.to_lowercase().as_str(), "heic" | "heif") {
                        acc.2 += 1; // Increment heic_count
                    }
                }

                // Process file to metadata (don't insert yet)
                match process_file_to_metadata(&path, photos_dir) {
                    Ok(photo_metadata) => {
                        acc.0.push(photo_metadata); // Collect successful metadata
                    }
                    Err(e) => {
                        warn!("Failed to process file {}: {}", path.display(), e);
                    }
                }
                acc
            },
        )
        .reduce(
            || (vec![], 0usize, 0usize), // Initial state for reduction
            |mut a, mut b| {
                a.0.append(&mut b.0); // Combine photo_metadata vectors
                a.1 += b.1; // Sum total_files
                a.2 += b.2; // Sum heic_count
                a
            },
        );

    let (all_photos, total_files, heic_count) = reduction_result;

    // Insert photos in batches for better performance
    let batch_size = 500;
    let mut successful_count = 0;

    if !silent_mode {
        info!("💾 Inserting {} photos into database in batches of {}...", all_photos.len(), batch_size);
    }

    for (batch_num, chunk) in all_photos.chunks(batch_size).enumerate() {
        match db.insert_photos_batch(chunk) {
            Ok(inserted) => {
                successful_count += inserted;
                if !silent_mode && batch_num % 10 == 0 {
                    info!("   Batch {}: {} photos inserted", batch_num + 1, inserted);
                }
            }
            Err(e) => {
                error!("Failed to insert batch {}: {}", batch_num + 1, e);
            }
        }
    }

    let processing_time = start_time.elapsed();
    let processing_secs = processing_time.as_secs_f64();
    let avg_time_per_file_ms = if total_files > 0 {
        (processing_secs * 1000.0) / total_files as f64
    } else {
        0.0
    };

    let final_count = successful_count;
    let gps_count = successful_count; // All successfully processed have GPS data
    let no_gps_count = total_files - successful_count;

    // Print processing statistics
    if !silent_mode {
        info!("\n📊 Статистика обработки:");
        info!("   🔍 Всего файлов проверено: {}", total_files);
        info!("   📸 Обработано фотографий: {}", final_count);
        info!("   🗺️  С GPS-данными: {}", gps_count);
        info!("   ❌ Без GPS: {}", no_gps_count);
        info!("   📱 HEIC файлов: {}", heic_count);
        info!(
            "   📷 JPEG/другие: {}",
            final_count.saturating_sub(heic_count)
        );
        info!("   ⏱️  Время обработки: {:.2} сек", processing_secs);
        info!(
            "   📈 Среднее время на файл: {:.1} мс",
            avg_time_per_file_ms
        );

        // Performance prediction for large collections
        if total_files >= 100 {
            let predicted_10k_time = (avg_time_per_file_ms * 10000.0) / 1000.0;
            let predicted_100k_time = (avg_time_per_file_ms * 100000.0) / 1000.0;

            info!("\n🔮 Прогноз производительности:");
            info!(
                "   📊 Для 10,000 фото: ~{:.1} минут",
                predicted_10k_time / 60.0
            );
            info!(
                "   📊 Для 100,000 фото: ~{:.1} минут",
                predicted_100k_time / 60.0
            );
            info!("   💡 On-demand генерация маркеров: ~0% времени на старте!");
            info!(
                "   💡 Экономия диска: {} файлов не создается",
                total_files * 2
            ); // ~2KB per saved thumbnail
        }

        info!("\n🎉 Обработка завершена! Данные сохранены в памяти.");
        info!(
            "   🗄️  База данных содержит {} фотографий с GPS-данными",
            final_count
        );
    }

    Ok((
        total_files,
        final_count,
        gps_count,
        no_gps_count,
        heic_count,
    ))
}

/// Simplified version of the function for backward compatibility
pub fn process_photos_into_database(db: &Database, photos_dir: &Path) -> Result<()> {
    process_photos_with_stats(db, photos_dir, true, true)?;
    Ok(())
}

/// Processes photos from the specified folder and sends progress events
pub fn process_photos_from_directory(
    db: &Database,
    photos_dir: &Path,
) -> Result<(usize, usize, usize, usize, usize)> {
    info!(
        "🔍 Processing photos from directory: {}",
        photos_dir.display()
    );

    // Use the new combined function, but without silent_mode
    process_photos_with_stats(db, photos_dir, false, true)
}

/// Processes a single file and returns PhotoMetadata (without inserting to DB)
fn process_file_to_metadata(path: &Path, photos_dir: &Path) -> Result<PhotoMetadata> {
    // Check the file extension, saving it in lowercase for checks
    let ext_lower = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Basic list of supported formats
    let supported_formats = [
        "jpg", "jpeg", "heic", "heif", "avif",
    ];

    if !supported_formats.contains(&ext_lower.as_str()) {
        anyhow::bail!("File is not a supported image");
    }

    // Check if it's HEIC or not, using the lowercase version
    let is_heif = matches!(ext_lower.as_str(), "heic" | "heif" | "avif");

    // --- GPS and date extraction ---
    let (lat, lng, datetime_opt) = if is_heif {
        // Try to extract metadata from HEIC
        extract_metadata_from_heic(path)?
    } else {
        // For standard formats, use our parsers
        if ext_lower == "jpg" || ext_lower == "jpeg" {
            // Use our own JPEG parser
            extract_metadata_from_jpeg(path)?
        } else {
            // For other formats (PNG, TIFF, etc.), keep the old method
            let file = fs::File::open(path)?;
            let mut bufreader = std::io::BufReader::new(&file);
            let exifreader = exif::Reader::new();
            let exif = exifreader.read_from_container(&mut bufreader)?;

            let lat = get_gps_coord(&exif, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef)?;
            let lng = get_gps_coord(&exif, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef)?;
            let datetime = get_datetime_from_exif(&exif);

            if lat.is_none() || lng.is_none() {
                anyhow::bail!("GPS data not found");
            }

            (lat.unwrap(), lng.unwrap(), datetime)
        }
    };

    let datetime_str = datetime_opt
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Unknown Date".to_string());

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

    Ok(PhotoMetadata {
        filename: filename.to_string(),
        relative_path,
        datetime: datetime_str,
        lat,
        lng,
        file_path: path.to_string_lossy().to_string(),
        is_heic: is_heif,
    })
}

