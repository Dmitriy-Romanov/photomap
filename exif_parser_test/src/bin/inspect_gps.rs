use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --bin inspect_gps <file_path>");
        std::process::exit(1);
    }

    let path = &args[1];
    println!("📸 Inspecting: {}", path);
    println!("\n=== REXIF (Reference) ===");
    
    match rexif::parse_file(path) {
        Ok(data) => {
            let mut found_gps = false;
            for entry in data.entries {
                let tag_name = format!("{:?}", entry.tag);
                if tag_name.contains("GPS") {
                    found_gps = true;
                    println!("{:?} = {:?}", entry.tag, entry.value);
                }
            }
            if !found_gps {
                println!("❌ No GPS tags found by rexif");
            }
        }
        Err(e) => {
            println!("❌ rexif failed: {}", e);
        }
    }

    println!("\n=== KAMADAK-EXIF (Our Parser) ===");
    match std::fs::File::open(path) {
        Ok(file) => {
            let mut bufreader = std::io::BufReader::new(&file);
            let mut exifreader = exif::Reader::new();
            exifreader.continue_on_error(true); // Tolerate non-standard EXIF
            match exifreader.read_from_container(&mut bufreader) {
                Ok(exif_data) => {
                    let mut found_gps = false;
                    for field in exif_data.fields() {
                        let tag_name = format!("{:?}", field.tag);
                        if tag_name.contains("GPS") {
                            found_gps = true;
                            println!("{:?} = {:?}", field.tag, field.value);
                        }
                    }
                    if !found_gps {
                        println!("❌ No GPS tags found by kamadak-exif");
                    }
                }
                Err(exif::Error::PartialResult(partial)) => {
                    let (exif_data, errors) = partial.into_inner();
                    println!("⚠️  Partial result with {} errors:", errors.len());
                    for err in &errors {
                        println!("   - {}", err);
                    }
                    let mut found_gps = false;
                    for field in exif_data.fields() {
                        let tag_name = format!("{:?}", field.tag);
                        if tag_name.contains("GPS") {
                            found_gps = true;
                            println!("{:?} = {:?}", field.tag, field.value);
                        }
                    }
                    if !found_gps {
                        println!("❌ No GPS tags found in partial result");
                    }
                }
                Err(e) => {
                    println!("❌ kamadak-exif failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to open file: {}", e);
        }
    }
}
