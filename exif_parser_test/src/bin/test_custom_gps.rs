use std::env;
use exif_parser_test::gps_parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --bin test_custom_gps <file_path>");
        std::process::exit(1);
    }

    let path = &args[1];
    println!("📸 Testing custom GPS parser on: {}", path);
    
    match gps_parser::extract_gps_from_malformed_exif(std::path::Path::new(path)) {
        Some((lat, lon)) => {
            println!("✅ GPS found: {}, {}", lat, lon);
        }
        None => {
            println!("❌ No GPS data found");
        }
    }
}
