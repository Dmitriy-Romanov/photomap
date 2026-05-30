use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

mod constants;
mod database;
mod exif_parser;
mod geocoding;
mod image_processing;
mod process_manager;
mod processing;
pub mod server;
mod settings;
mod utils;

use database::Database;
use libheif_rs::integration::image::register_all_decoding_hooks;
use server::state::AppState;
use settings::Settings;

#[tokio::main]
async fn main() -> Result<()> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    println!("---");
    println!("🚀 Session start: PhotoMap Processor v{}", VERSION);
    println!("---");

    register_all_decoding_hooks();

    let mut port = 3001;
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                if i + 1 < args.len() {
                    if let Ok(p) = args[i + 1].parse::<u16>() {
                        port = p;
                    } else {
                        eprintln!("⚠️ Invalid port value: {}", args[i + 1]);
                        std::process::exit(1);
                    }
                    i += 2;
                } else {
                    eprintln!("⚠️ Missing value for --port");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                println!("PhotoMap Processor v{}", VERSION);
                println!("Parallel photo processing, EXIF metadata extraction and interactive map server.");
                println!();
                println!("Usage:");
                println!("  photomap_processor [options]");
                println!();
                println!("Options:");
                println!("  -p, --port <port>  Specify port number (default: 3001)");
                println!("  -h, --help         Show this help message");
                return Ok(());
            }
            _ => {
                eprintln!("⚠️ Unknown argument: {}", args[i]);
                eprintln!("Run with --help to see available options.");
                std::process::exit(1);
            }
        }
    }

    process_manager::ensure_single_instance()?;

    println!("🗄️ Initializing database (In-Memory)...");
    let db = Database::new().with_context(|| "Failed to initialize database")?;
    println!("✅ Database initialized successfully");

    std::thread::spawn(|| {
        geocoding::ReverseGeocoder::init();
    });

    println!(" 🚀 Starting HTTP server for on-demand marker generation");

    let (event_sender, event_sender_receiver) = tokio::sync::mpsc::channel(100);
    let event_broadcast = tokio::sync::broadcast::channel(100).0;
    let (shutdown_sender, _shutdown_receiver) = tokio::sync::broadcast::channel(1);
    let event_broadcast_fwd = event_broadcast.clone();
    tokio::spawn(async move {
        let mut rx = event_sender_receiver;
        while let Some(event) = rx.recv().await {
            let _ = event_broadcast_fwd.send(event);
        }
    });

    let settings = Arc::new(Mutex::new(Settings::load()?));
    println!(
        " ⚙️ Config file loaded from: {}",
        Settings::config_path().display()
    );

    let folder_paths: Vec<String> = {
        let guard = settings.lock().await;
        guard
            .folders
            .iter()
            .filter_map(|f| f.as_ref().cloned())
            .collect()
    };

    if !folder_paths.is_empty() {
        match db.load_from_disk(&folder_paths) {
            Ok(true) => {
                let count = db.get_photos_count().unwrap_or(0);
                println!("✅ Loaded {} photos from cache (paths match)", count);
            }
            _ => {
                println!(
                    "🚀 Cache miss or mismatch. Processing {} folder(s)...",
                    folder_paths.len()
                );
                let _ = db.clear_all_photos();

                for folder_path in &folder_paths {
                    let photos_path = Path::new(folder_path);
                    if !photos_path.exists() {
                        eprintln!("⚠️ Saved folder not found: {}", folder_path);
                        continue;
                    }
                    println!("📂 Processing saved folder: {}", folder_path);
                    if let Err(e) =
                        processing::process_photos_with_stats(&db, photos_path, false, false)
                    {
                        eprintln!("⚠️ Error processing {}: {}", folder_path, e);
                    }
                }

                let count = db.get_photos_count().unwrap_or(0);
                println!("✅ Total photos in database: {}", count);

                if let Err(e) = db.save_to_disk(&folder_paths) {
                    eprintln!("⚠️ Failed to save cache: {}", e);
                } else {
                    println!("💾 Cache saved successfully");
                }
            }
        }
    } else {
        println!("ℹ️ No saved folders found. Please select folders using the web interface");
    }

    let app_state = AppState {
        db,
        settings: settings.clone(),
        event_sender,
        event_broadcast,
        shutdown_sender,
    };

    {
        let guard = settings.lock().await;
        if guard.start_browser {
            let url = format!("http://127.0.0.1:{}", port);
            println!(" 🌐 Opening browser at {}", url);
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if let Err(e) = utils::open_browser(&url) {
                    eprintln!("Failed to open browser: {}", e);
                }
            });
        }
    }

    server::start_server(app_state, port).await?;
    Ok(())
}
