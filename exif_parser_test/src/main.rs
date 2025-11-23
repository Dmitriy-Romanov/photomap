use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Write, Read, Seek};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() -> Result<()> {
    println!("🚀 Starting Exif Parser Test...");

    // 1. Select folder
    let folder = rfd::FileDialog::new()
        .set_title("Select folder with photos")
        .pick_folder()
        .context("No folder selected")?;

    println!("📂 Scanning folder: {}", folder.display());
    println!("🔍 Processing files...\n");

    // 2. Open log files
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("failures.txt")?;
        
    let mut accuracy_log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("accuracy_issues.txt")?;

    let mut count_processed = 0;
    let mut count_failures = 0;
    let mut count_accuracy_issues = 0;
    
    const COORD_TOLERANCE: f64 = 0.0001; // ~11 метров

    // 3. Process files on-the-fly
    for entry in WalkDir::new(&folder).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            let ext_lower = ext.to_lowercase();
            if ext_lower == "jpg" || ext_lower == "jpeg" || 
               ext_lower == "heic" || ext_lower == "heif" || 
               ext_lower == "avif" {
                
                count_processed += 1;
                
                // Show progress
                print!("\rProcessing file #{}: {} ... ", 
                       count_processed,
                       path.file_name().unwrap_or_default().to_str().unwrap_or("???"));
                std::io::stdout().flush().ok();

                // Try our parser
                let our_result = extract_gps_our(path);
                
                if our_result.is_none() {
                    // Our parser failed - check if exiftool finds GPS
                    if let Some(exiftool_gps) = extract_gps_exiftool(path) {
                        // FAILURE: exiftool found GPS but we didn't
                        println!("\n⚠️  MISSING GPS: {}", path.display());
                        println!("  Our parser: ✗ FAILED");
                        println!("  exiftool: ✓ ({:.6}, {:.6})", exiftool_gps.0, exiftool_gps.1);
                        writeln!(log_file, "{}", path.display())?;
                        count_failures += 1;
                    }
                } else if let Some(our_gps) = our_result {
                    // Our parser succeeded - verify accuracy against exiftool
                    if let Some(exiftool_gps) = extract_gps_exiftool(path) {
                        let lat_diff = (our_gps.0 - exiftool_gps.0).abs();
                        let lon_diff = (our_gps.1 - exiftool_gps.1).abs();
                        
                        if lat_diff > COORD_TOLERANCE || lon_diff > COORD_TOLERANCE {
                            // ACCURACY ISSUE: coordinates don't match
                            println!("\n⚠️  ACCURACY ISSUE: {}", path.display());
                            println!("  Our parser: ({:.6}, {:.6})", our_gps.0, our_gps.1);
                            println!("  exiftool:   ({:.6}, {:.6})", exiftool_gps.0, exiftool_gps.1);
                            println!("  Difference: Δlat={:.6}°, Δlon={:.6}°", lat_diff, lon_diff);
                            writeln!(accuracy_log, "{} | Our: ({:.6}, {:.6}) | exiftool: ({:.6}, {:.6}) | Diff: ({:.6}, {:.6})",
                                     path.display(), our_gps.0, our_gps.1, exiftool_gps.0, exiftool_gps.1, lat_diff, lon_diff)?;
                            count_accuracy_issues += 1;
                        }
                    }
                }
            }
        }
    }

    println!("\n\n✅ Scan complete.");
    println!("Total processed: {}", count_processed);
    println!("Missing GPS (we failed, exiftool succeeded): {}", count_failures);
    println!("Accuracy issues (coordinates mismatch): {}", count_accuracy_issues);
    println!("\nSee failures.txt for missing GPS files.");
    println!("See accuracy_issues.txt for coordinate mismatches.");
    
    // Copy failed files to 'JPG for checks' directory
    if count_failures > 0 {
        println!("\n📋 Copying failed files to 'JPG for checks' directory...");
        let target_dir = PathBuf::from("/Users/dmitriiromanov/claude/photomap/exif_parser_test/JPG for checks");
        
        // Create target directory if it doesn't exist
        std::fs::create_dir_all(&target_dir)
            .with_context(|| "Failed to create 'JPG for checks' directory")?;
        
        // Read failures.txt and copy each file
        let failures_content = std::fs::read_to_string("failures.txt")
            .with_context(|| "Failed to read failures.txt")?;
        
        let mut copied = 0;
        for line in failures_content.lines() {
            let source_path = PathBuf::from(line.trim());
            if source_path.exists() {
                if let Some(filename) = source_path.file_name() {
                    let target_path = target_dir.join(filename);
                    match std::fs::copy(&source_path, &target_path) {
                        Ok(_) => {
                            copied += 1;
                            println!("  ✓ Copied: {}", filename.to_string_lossy());
                        }
                        Err(e) => {
                            println!("  ✗ Failed to copy {}: {}", filename.to_string_lossy(), e);
                        }
                    }
                }
            }
        }
        
        println!("📦 Copied {} of {} failed files.", copied, count_failures);
    }

    Ok(())
}

// --- "Our" Code (Ported from Photomap) ---
fn extract_gps_our(path: &Path) -> Option<(f64, f64)> {
    let ext = path.extension()?.to_str()?.to_lowercase();

    if ext == "heic" || ext == "heif" || ext == "avif" {
        // HEIC/HEIF/AVIF logic (identical to PhotoMap)
        let heic_result = (|| -> Option<(f64, f64)> {
            let ctx = libheif_rs::HeifContext::read_from_file(path.to_str()?).ok()?;
            let handle = ctx.primary_image_handle().ok()?;
            
            let count = handle.number_of_metadata_blocks(0);
            if count == 0 {
                return None;
            }
            
            let mut ids = vec![0; count as usize];
            let count = handle.metadata_block_ids(&mut ids, 0);
            
            for &id in ids.iter().take(count) {
                if let Some(type_str) = handle.metadata_type(id) {
                    if type_str == "Exif" {
                        if let Ok(data) = handle.metadata(id) {
                            // Skip "Exif\0\0" header if present
                            let tiff_start = if data.len() > 4 && data[4..].starts_with(b"Exif\0\0") {
                                10
                            } else if data.starts_with(b"Exif\0\0") {
                                6
                            } else {
                                0
                            };
                            
                            if data.len() > tiff_start {
                                if let Ok(exif_reader) = exif::Reader::new().read_raw(data[tiff_start..].to_vec()) {
                                    return parse_exif_gps(&exif_reader);
                                }
                            }
                        }
                    }
                }
            }
            None
        })();
        
        if heic_result.is_some() {
            return heic_result;
        }
        
        // Fallback: Check if it's JPEG disguised as HEIC (Xiaomi bug)
        let mut file = File::open(path).ok()?;
        let mut buffer = [0u8; 2];
        if file.read_exact(&mut buffer).is_ok() && buffer == [0xFF, 0xD8] {
            // It's a JPEG! Rewind and parse as JPEG
            file.seek(std::io::SeekFrom::Start(0)).ok()?;
            let mut bufreader = BufReader::new(file);
            let exif_reader = exif::Reader::new();
            if let Ok(exif_data) = exif_reader.read_from_container(&mut bufreader) {
                return parse_exif_gps(&exif_data);
            }
        }
        
        return None;
    }
    
    // JPG/TIFF logic using kamadak-exif
    let file = File::open(path).ok()?;
    let mut bufreader = BufReader::new(file);
    let mut exif_reader = exif::Reader::new();
    exif_reader.continue_on_error(true);
    
    match exif_reader.read_from_container(&mut bufreader) {
        Ok(exif_data) => {
            if let Some(gps) = parse_exif_gps(&exif_data) {
                return Some(gps);
            }
        }
        Err(exif::Error::PartialResult(partial)) => {
            let (exif_data, _errors) = partial.into_inner();
            if let Some(gps) = parse_exif_gps(&exif_data) {
                return Some(gps);
            }
        }
        _ => {}
    }
    
    None
}

fn parse_exif_gps(exif_data: &exif::Exif) -> Option<(f64, f64)> {
    // Extract both coordinates using the improved logic
    let lat = extract_single_gps_coord(exif_data, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef)?;
    let lon = extract_single_gps_coord(exif_data, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef)?;
    Some((lat, lon))
}

fn extract_single_gps_coord(exif_data: &exif::Exif, coord_tag: exif::Tag, ref_tag: exif::Tag) -> Option<f64> {
    // Try PRIMARY IFD first (most common location)
    if let Some(result) = try_extract_from_ifd(exif_data, coord_tag, ref_tag, exif::In::PRIMARY) {
        return Some(result);
    }
    
    // Fallback: Search through ALL fields to find GPS data
    // Some cameras (like Samsung) may store GPS in different IFDs
    for field in exif_data.fields() {
        if field.tag == coord_tag {
            // Found coordinate field - now find its reference
            for ref_field in exif_data.fields() {
                if ref_field.tag == ref_tag && ref_field.ifd_num == field.ifd_num {
                    // Found matching reference in same IFD
                    if let exif::Value::Rational(vec) = &field.value {
                        if vec.len() == 3 {
                            let degrees = vec[0].num as f64 / vec[0].denom as f64;
                            let minutes = vec[1].num as f64 / vec[1].denom as f64;
                            let seconds = vec[2].num as f64 / vec[2].denom as f64;
                            let mut decimal = degrees + minutes / 60.0 + seconds / 3600.0;

                            // Apply reference (S/W are negative values)
                            if let exif::Value::Ascii(refs) = &ref_field.value {
                                if let Some(s) = refs.first() {
                                    if let Ok(s_str) = std::str::from_utf8(s) {
                                        if s_str.starts_with('S') || s_str.starts_with('W') {
                                            decimal = -decimal;
                                        }
                                    }
                                }
                            }
                            return Some(decimal);
                        }
                    }
                }
            }
        }
    }
    
    None
}

fn try_extract_from_ifd(exif_data: &exif::Exif, coord_tag: exif::Tag, ref_tag: exif::Tag, ifd: exif::In) -> Option<f64> {
    let coord_field = exif_data.get_field(coord_tag, ifd)?;
    let ref_field = exif_data.get_field(ref_tag, ifd)?;

    if let exif::Value::Rational(rationals) = &coord_field.value {
        if rationals.len() == 3 {
            let degrees = rationals[0].num as f64 / rationals[0].denom as f64;
            let minutes = rationals[1].num as f64 / rationals[1].denom as f64;
            let seconds = rationals[2].num as f64 / rationals[2].denom as f64;
            let mut decimal = degrees + minutes / 60.0 + seconds / 3600.0;

            if let exif::Value::Ascii(refs) = &ref_field.value {
                if let Some(s) = refs.first() {
                    if let Ok(s_str) = std::str::from_utf8(s) {
                        if s_str.starts_with('S') || s_str.starts_with('W') {
                            decimal = -decimal;
                        }
                    }
                }
            }
            return Some(decimal);
        }
    }
    None
}


// --- "Exiftool" Code (Gold Standard - 99.99% accuracy) ---
fn extract_gps_exiftool(path: &Path) -> Option<(f64, f64)> {
    use std::process::Command;
    
    // Run exiftool to extract GPS coordinates
    let output = Command::new("exiftool")
        .arg("-GPSLatitude")
        .arg("-GPSLongitude")
        .arg("-n")  // Numerical output for GPS coordinates
        .arg(path)
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lat: Option<f64> = None;
    let mut lon: Option<f64> = None;
    
    for line in stdout.lines() {
        let line_trimmed = line.trim();
        
        // Format: "GPS Latitude                    : 48.8725955"
        if line_trimmed.starts_with("GPS Latitude") && line_trimmed.contains(':') {
            if let Some(value_part) = line_trimmed.split(':').nth(1) {
                let parsed = value_part.trim().parse();
                lat = parsed.ok();
            }
        } else if line_trimmed.starts_with("GPS Longitude") && line_trimmed.contains(':') {
            if let Some(value_part) = line_trimmed.split(':').nth(1) {
                let parsed = value_part.trim().parse();
                lon = parsed.ok();
            }
        }
    }
    
    if let (Some(lat_val), Some(lon_val)) = (lat, lon) {
        return Some((lat_val, lon_val));
    }
    
    None
}
