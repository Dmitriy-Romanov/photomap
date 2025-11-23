use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Write, Read, Seek};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() -> Result<()> {
    println!("🚀 Starting Exif Parser Test...");

    // 1. Select folder
    let args: Vec<String> = std::env::args().collect();
    let folder = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        rfd::FileDialog::new()
            .set_title("Select folder to scan")
            .pick_folder()
            .context("No folder selected")?
    };

    println!("📂 Scanning folder: {}", folder.display());

    // Prepare failures log file
    let log_path = Path::new("failures.txt");
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)
        .context("Failed to create failures.txt")?;

    let supported_extensions = ["jpg", "jpeg", "heic", "heif", "avif"];
    let mut count_processed = 0;
    let mut count_failures = 0;

    // 2. Recursive scan
    for entry in WalkDir::new(&folder).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            let ext_lower = ext.to_lowercase();
            if supported_extensions.contains(&ext_lower.as_str()) {
                count_processed += 1;
                print!("\rProcessing file #{}: {} ... ", count_processed, path.file_name().unwrap().to_string_lossy());
                std::io::stdout().flush()?;

                // 3. Try "Our" code
                let our_result = extract_gps_our(path);
                
                println!("\nTesting: {}", path.display());
                println!("  Our parser: {}", if our_result.is_some() { "✓ SUCCESS" } else { "✗ FAILED" });

                if our_result.is_none() {
                    // 4. If "Our" code failed, try exiftool (gold standard)
                    if let Some(exiftool_gps) = extract_gps_exiftool(path) {
                        // exiftool succeeded where we failed!
                        println!("  exiftool: ✓ SUCCESS ({:.6}, {:.6})", exiftool_gps.0, exiftool_gps.1);
                        println!("\n⚠️  FAILURE DETECTED: {}", path.display());
                        writeln!(log_file, "{}", path.display())?;
                        count_failures += 1;
                    } else {
                        println!("  exiftool: ✗ FAILED");
                    }
                } else {
                    println!("  (skipped exiftool - our parser succeeded)");
                }
            }
        }
    }

    println!("\n\n✅ Scan complete.");
    println!("Total processed: {}", count_processed);
    println!("Failures found: {}", count_failures);
    println!("See failures.txt for details.");
    
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
        // Skip HEIC/HEIF/AVIF files in test tool - focus on JPEG
        // Main PhotoMap has full HEIC support via libheif-rs
        return None;
    }
    
    // JPG/TIFF logic using kamadak-exif
    let file = File::open(path).ok()?;
    let mut bufreader = BufReader::new(file);
    // Enable continue_on_error to handle Lightroom-processed files with non-standard EXIF
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
    
    // DON'T use fallback parser in test - we want to find files where basic parser fails!
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
