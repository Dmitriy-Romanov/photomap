use anyhow::{Context, Result};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Write, Read, Seek};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod gps_parser;

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

                if our_result.is_none() {
                    // 4. If "Our" code failed, try "Reference" code
                    if let Some(_ref_gps) = extract_gps_ref(path) {
                        // Reference succeeded where we failed!
                        println!("\n⚠️  FAILURE DETECTED: {}", path.display());
                        writeln!(log_file, "{}", path.display())?;
                        count_failures += 1;
                    }
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

    if ext == "heic" || ext == "heif" {
        // HEIC logic (simplified for this test, assuming libheif-rs usage)
        // In real photomap we use libheif_rs::HeifContext etc.
        // For this test, we'll try to implement basic HEIC reading if possible,
        // or just return None if we can't easily port the full HEIC logic without more setup.
        // NOTE: Photomap uses libheif-rs. Let's try to use it here too.
        use libheif_rs::HeifContext;
        // Try to open as HEIC first
        if let Ok(ctx) = HeifContext::read_from_file(path.to_str()?) {
            if let Ok(handle) = ctx.primary_image_handle() {
                 // libheif-rs doesn't expose easy metadata reading in all versions, 
                 // but let's assume we can get EXIF block.
                 // Actually, for this test, let's focus on JPG first as it's most common for failures.
                 // If HEIC is needed, we need to copy the exact logic.
                 // Correct usage of libheif-rs metadata API
                 // Pass 0 for type_filter to match all types (0 implements Into<FourCC>)
                 let count = handle.number_of_metadata_blocks(0);
                 if count > 0 {
                     let mut ids = vec![0; count as usize];
                     let count = handle.metadata_block_ids(&mut ids, 0);
                     for &id in ids.iter().take(count) {
                         if let Some(type_str) = handle.metadata_type(id) {
                             if type_str == "Exif" {
                                 if let Ok(data) = handle.metadata(id) {
                                     if let Ok(exif_reader) = exif::Reader::new().read_raw(data) {
                                         return parse_exif_gps(&exif_reader);
                                     }
                                 }
                             }
                         }
                     }
                 }
            }
        }
        
        // Fallback: Check if it's actually a JPEG disguised as HEIC
        // This happens with some Xiaomi phones
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
    } else {
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
        
        // Fallback to custom GPS parser for malformed EXIF files (e.g., Lightroom-processed)
        return gps_parser::extract_gps_from_malformed_exif(path);
    }
    None
}

fn parse_exif_gps(exif_data: &exif::Exif) -> Option<(f64, f64)> {
    let lat_field = exif_data.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY);
    let lat_ref_field = exif_data.get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY);
    let lon_field = exif_data.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY);
    let lon_ref_field = exif_data.get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY);

    if let (Some(lat), Some(lat_ref), Some(lon), Some(lon_ref)) =
        (lat_field, lat_ref_field, lon_field, lon_ref_field)
    {
        let lat_val = convert_dm_s_to_decimal(lat, lat_ref)?;
        let lon_val = convert_dm_s_to_decimal(lon, lon_ref)?;
        return Some((lat_val, lon_val));
    }
    None
}

fn convert_dm_s_to_decimal(field: &exif::Field, ref_field: &exif::Field) -> Option<f64> {
    match field.value {
        exif::Value::Rational(ref rationals) if rationals.len() == 3 => {
            let degrees = rationals[0].num as f64 / rationals[0].denom as f64;
            let minutes = rationals[1].num as f64 / rationals[1].denom as f64;
            let seconds = rationals[2].num as f64 / rationals[2].denom as f64;
            let mut decimal = degrees + minutes / 60.0 + seconds / 3600.0;

            if let exif::Value::Ascii(ref refs) = ref_field.value {
                if let Some(s) = refs.first() {
                    let s = std::str::from_utf8(s).ok()?;
                    if s.starts_with('S') || s.starts_with('W') {
                        decimal = -decimal;
                    }
                }
            }
            Some(decimal)
        }
        _ => None,
    }
}


// --- "Reference" Code (using rexif) ---
fn extract_gps_ref(path: &Path) -> Option<(f64, f64)> {
    // Rexif is a pure Rust library, good for double-checking
    if let Ok(data) = rexif::parse_file(path.to_str()?) {
        let mut lat: Option<f64> = None;
        let mut lon: Option<f64> = None;
        
        for entry in data.entries {
            match entry.tag {
                rexif::ExifTag::GPSLatitude => {
                    if let rexif::TagValue::URational(ref v) = entry.value {
                         if v.len() == 3 {
                             lat = Some(v[0].value() + v[1].value() / 60.0 + v[2].value() / 3600.0);
                         }
                    }
                },
                rexif::ExifTag::GPSLatitudeRef => {
                     if let rexif::TagValue::Ascii(ref s) = entry.value {
                         if s.starts_with("S") && lat.is_some() {
                             lat = Some(-lat.unwrap());
                         }
                     }
                },
                rexif::ExifTag::GPSLongitude => {
                    if let rexif::TagValue::URational(ref v) = entry.value {
                         if v.len() == 3 {
                             lon = Some(v[0].value() + v[1].value() / 60.0 + v[2].value() / 3600.0);
                         }
                    }
                },
                rexif::ExifTag::GPSLongitudeRef => {
                     if let rexif::TagValue::Ascii(ref s) = entry.value {
                         if s.starts_with("W") && lon.is_some() {
                             lon = Some(-lon.unwrap());
                         }
                     }
                },
                _ => {}
            }
        }
        
        if let (Some(l1), Some(l2)) = (lat, lon) {
            return Some((l1, l2));
        }
    }
    None
}
