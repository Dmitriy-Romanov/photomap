use crate::constants::{is_heic_format, is_supported_image};
use crate::database::{Database, PhotoMetadata};
use crate::exif_parser::{
    extract_metadata_from_heic, extract_metadata_from_jpeg, get_datetime_string, get_gps_coord,
};
use anyhow::Result;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Recursively walks a directory collecting image files
fn walk_dir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut dirs_to_visit = vec![dir.to_path_buf()];

    while let Some(current_dir) = dirs_to_visit.pop() {
        if let Ok(entries) = fs::read_dir(&current_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Skip hidden directories and common ignore patterns
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if !name.starts_with('.')
                            && name != "node_modules"
                            && name != "target"
                            && name != ".git"
                        {
                            dirs_to_visit.push(path);
                        }
                    }
                } else if path.is_file() {
                    files.push(path);
                }
            }
        }
    }
    files
}

/// Processes photos and saves metadata to the database
/// Returns processing statistics: (total_files, processed_count, no_gps_count, heic_count)
pub fn process_photos_with_stats(
    db: &Database,
    photos_dir: &Path,
    silent_mode: bool,
    clear_database: bool,
) -> Result<(usize, usize, usize, usize)> {
    if !silent_mode {
        println!("🔍 Scanning photos directory: {}", photos_dir.display());
    }

    if !photos_dir.exists() {
        let error_msg = format!("❌ Photos directory not found: {}", photos_dir.display());
        if silent_mode {
            return Err(anyhow::Error::msg(error_msg));
        } else {
            eprintln!("{}", error_msg);
            return Ok((0, 0, 0, 0));
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

    // Collect all image files using custom walk function
    let all_files = walk_dir(photos_dir);

    // Process files in parallel using Rayon with timing
    let start_time = std::time::Instant::now();

    if !silent_mode {
        println!("📊 Starting parallel processing of files...");
    }

    let reduction_result = all_files
        .into_par_iter()  // Rayon parallel iterator
        .filter(|path| {
            // Filter by extension - only process supported image formats
            path.extension()
                .and_then(|s| s.to_str())
                .map(|ext| is_supported_image(ext))
                .unwrap_or(false)
        })
        .fold(
            || (vec![], 0usize, 0usize), // Initial state for each thread: (photo_metadata_vec, total_files, heic_count)
            |mut acc, path: PathBuf| {
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
                        let err_msg = e.to_string();
                        if err_msg.contains("GPS data not found") {
                            println!("ℹ️  Skipped {}: No GPS data", path.display());
                        } else {
                            eprintln!("Failed to process file {}: {}", path.display(), e);
                        }
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
    let mut successful_count = 0;

    // Insert all photos into database at once
    if !silent_mode {
        println!("💾 Inserting {} photos into database...", all_photos.len());
    }

    match db.insert_photos_batch(&all_photos) {
        Ok(inserted) => {
            successful_count = inserted;
            if !silent_mode {
                println!("✅ Successfully inserted {} photos", inserted);
            }
        }
        Err(e) => {
            eprintln!("Failed to insert photos: {}", e);
        }
    }

    let processing_time = start_time.elapsed();
    let processing_secs = processing_time.as_secs_f64();
    let avg_time_per_file_ms = if total_files > 0 {
        (processing_secs * 1000.0) / total_files as f64
    } else {
        0.0
    };

    let no_gps_count = total_files - successful_count;

    // Print processing statistics
    if !silent_mode {
        println!("\n📊 Processing Statistics:");
        println!("   🔍 Total files checked: {}", total_files);
        println!("   📸 Photos with GPS: {}", successful_count);
        println!("   ❌ Without GPS: {}", no_gps_count);
        println!("   📱 HEIC files: {}", heic_count);
        println!(
            "   📷 JPEG/other: {}",
            successful_count.saturating_sub(heic_count)
        );
        println!("   ⏱️  Processing time: {:.2} sec", processing_secs);
        println!(
            "   📈 Average time per file: {:.1} ms",
            avg_time_per_file_ms
        );

        println!("\n🎉 Processing complete! Data stored in memory.");
        println!(
            "   🗄️  Database contains {} photos with GPS data",
            successful_count
        );

    }

    // Note: Cache is saved manually by caller (main.rs) with all folder paths

    Ok((
        total_files,
        successful_count,
        no_gps_count,
        heic_count,
    ))
}

/// Processes photos from the specified folder and sends progress events
pub fn process_photos_from_directory(
    db: &Database,
    photos_dir: &Path,
) -> Result<(usize, usize, usize, usize)> {
    println!(
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

    // Check if file format is supported
    if !is_supported_image(&ext_lower) {
        anyhow::bail!("File is not a supported image");
    }

    // Check if it's HEIC/HEIF format
    let is_heif = is_heic_format(&ext_lower);

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
            // Fallback for other formats with EXIF
            let file = fs::File::open(path)?;
            let mut bufreader = std::io::BufReader::new(&file);
            let exifreader = exif::Reader::new();
            let exif = exifreader.read_from_container(&mut bufreader)?;

            let lat = get_gps_coord(&exif, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef)?;
            let lng = get_gps_coord(&exif, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef)?;
            let datetime = get_datetime_string(&exif);

            if lat.is_none() || lng.is_none() {
                anyhow::bail!("GPS data not found");
            }

            (lat.unwrap(), lng.unwrap(), datetime)
        }
    };

    let datetime_str = datetime_opt.unwrap_or_else(|| "Unknown Date".to_string());

    // --- Create a database record ---
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::Error::msg("Invalid file name"))?;

    // Generate relative path from photos directory
    let relative_path = path
        .strip_prefix(photos_dir)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
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

