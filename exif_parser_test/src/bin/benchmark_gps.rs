use std::time::Instant;
use std::path::Path;
use exif_parser_test::gps_parser;

fn main() {
    let test_dir = "/Users/dmitriiromanov/claude/photomap/exif_parser_test/JPG for checks";
    
    // Collect all test files
    let files: Vec<_> = std::fs::read_dir(test_dir)
        .expect("Failed to read directory")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase() == "jpg")
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();
    
    println!("📊 Benchmarking GPS parsers on {} files", files.len());
    println!("🔄 Running 10 iterations per file...\n");
    
    // Benchmark our custom parser
    let start = Instant::now();
    for _ in 0..10 {
        for file in &files {
            let _ = gps_parser::extract_gps_from_malformed_exif(file);
        }
    }
    let custom_duration = start.elapsed();
    
    // Benchmark rexif
    let start = Instant::now();
    for _ in 0..10 {
        for file in &files {
            let _ = extract_gps_rexif(file);
        }
    }
    let rexif_duration = start.elapsed();
    
    // Results
    let total_ops = files.len() * 10;
    println!("📈 Results ({} total operations):", total_ops);
    println!();
    println!("🔧 Custom GPS Parser:");
    println!("   Total time: {:?}", custom_duration);
    println!("   Avg per file: {:.2} µs", custom_duration.as_micros() as f64 / total_ops as f64);
    println!();
    println!("📦 rexif:");
    println!("   Total time: {:?}", rexif_duration);
    println!("   Avg per file: {:.2} µs", rexif_duration.as_micros() as f64 / total_ops as f64);
    println!();
    
    if custom_duration < rexif_duration {
        let speedup = rexif_duration.as_micros() as f64 / custom_duration.as_micros() as f64;
        println!("🏆 Custom parser is {:.2}x FASTER!", speedup);
    } else {
        let slowdown = custom_duration.as_micros() as f64 / rexif_duration.as_micros() as f64;
        println!("⚠️  Custom parser is {:.2}x SLOWER", slowdown);
    }
}

fn extract_gps_rexif(path: &Path) -> Option<(f64, f64)> {
    let exif_data = rexif::parse_file(path).ok()?;
    
    let mut lat: Option<f64> = None;
    let mut lat_ref: Option<char> = None;
    let mut lon: Option<f64> = None;
    let mut lon_ref: Option<char> = None;
    
    for entry in exif_data.entries {
        match entry.tag {
            rexif::ExifTag::GPSLatitudeRef => {
                if let rexif::TagValue::Ascii(ref s) = entry.value {
                    lat_ref = s.chars().next();
                }
            }
            rexif::ExifTag::GPSLatitude => {
                if let rexif::TagValue::URational(ref coords) = entry.value {
                    if coords.len() == 3 {
                        let deg = coords[0].numerator as f64 / coords[0].denominator as f64;
                        let min = coords[1].numerator as f64 / coords[1].denominator as f64;
                        let sec = coords[2].numerator as f64 / coords[2].denominator as f64;
                        lat = Some(deg + min / 60.0 + sec / 3600.0);
                    }
                }
            }
            rexif::ExifTag::GPSLongitudeRef => {
                if let rexif::TagValue::Ascii(ref s) = entry.value {
                    lon_ref = s.chars().next();
                }
            }
            rexif::ExifTag::GPSLongitude => {
                if let rexif::TagValue::URational(ref coords) = entry.value {
                    if coords.len() == 3 {
                        let deg = coords[0].numerator as f64 / coords[0].denominator as f64;
                        let min = coords[1].numerator as f64 / coords[1].denominator as f64;
                        let sec = coords[2].numerator as f64 / coords[2].denominator as f64;
                        lon = Some(deg + min / 60.0 + sec / 3600.0);
                    }
                }
            }
            _ => {}
        }
    }
    
    let mut final_lat = lat?;
    let mut final_lon = lon?;
    
    if lat_ref == Some('S') {
        final_lat = -final_lat;
    }
    if lon_ref == Some('W') {
        final_lon = -final_lon;
    }
    
    Some((final_lat, final_lon))
}
