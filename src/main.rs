use anyhow::{Context, Result};
use std::path::Path;
use std::sync::{Arc, Mutex};

// Import modules
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
    // === Log Session Start ===
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    println!("---");
    println!("🚀 Session start: PhotoMap Processor v{}", VERSION);
    println!("---");

    // Register HEIC/HEIF decoder
    register_all_decoding_hooks();

    // Ensure single instance - kill existing processes
    process_manager::ensure_single_instance()?;

    // Initialize database
    println!("🗄️  Initializing database (In-Memory)...");
    let db = Database::new().with_context(|| "Failed to initialize database")?;
    println!("✅ Database initialized successfully");

    // Initialize Reverse Geocoder (Lazy load in background)
    println!("🌍 Initializing Reverse Geocoder...");
    std::thread::spawn(|| {
        geocoding::ReverseGeocoder::init();
    });

    println!("   🚀 Starting HTTP server for on-demand marker generation");

    // Start HTTP server
    let (event_sender, _event_receiver) = tokio::sync::broadcast::channel(100);
    let (shutdown_sender, _shutdown_receiver) = tokio::sync::broadcast::channel(1);

    let settings = Arc::new(Mutex::new(Settings::load()?));
    println!("   ⚙️  Config file loaded from: {}", Settings::config_path().display());

    // Process photos from saved folders if available
    {
        let settings_guard = settings.lock().unwrap();

        // Collect non-empty folder paths
        let folder_paths: Vec<String> = settings_guard.folders
            .iter()
            .filter_map(|f| f.as_ref().cloned())
            .collect();


        if !folder_paths.is_empty() {
            // Try to load from cache first
            let cache_loaded = match db.load_from_disk(&folder_paths) {
                Ok(loaded) => loaded,
                Err(e) => {
                    eprintln!("⚠️  Failed to load cache: {}", e);
                    false
                }
            };

            if cache_loaded {
                let count = db.get_photos_count().unwrap_or(0);
                println!("✅ Loaded {} photos from cache (paths match)", count);
            } else {
                println!("🚀 Cache miss or mismatch. Processing {} folder(s)...", folder_paths.len());

                // Clear database once before processing all folders
                if let Err(e) = db.clear_all_photos() {
                    eprintln!("⚠️  Failed to clear database: {}", e);
                }

                for folder_path in &folder_paths {
                    let photos_path = Path::new(folder_path);
                    if !photos_path.exists() {
                        eprintln!("⚠️  Saved folder not found: {}", folder_path);
                        continue;
                    }

                    println!("📂 Processing saved folder: {}", folder_path);

                    // Process without clearing (DB already cleared once above)
                    match processing::process_photos_with_stats(&db, photos_path, false, false) {
                        Ok(_) => {},
                        Err(e) => eprintln!("⚠️  Error processing {}: {}", folder_path, e),
                    }
                }

                let count = db.get_photos_count().unwrap_or(0);
                println!("✅ Total photos in database: {}", count);

                // Save to cache
                if let Err(e) = db.save_to_disk(&folder_paths) {
                    eprintln!("⚠️  Failed to save cache: {}", e);
                } else {
                    println!("💾 Cache saved successfully");
                }
            }
        } else {
            println!("ℹ️  No saved folders found. Please select folders using the web interface");
        }
    } // Release the lock

    let app_state = AppState {
        db,
        settings: settings.clone(),
        event_sender,
        shutdown_sender,
    };

    // Open browser if enabled in settings
    {
        let settings_guard = settings.lock().unwrap();
        if settings_guard.start_browser {
            let url = "http://127.0.0.1:3001";
            println!("   🌐 Opening browser at {}", url);

            // Spawn a task to open the browser after a short delay to ensure server is up
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if let Err(e) = utils::open_browser(url) {
                    eprintln!("Failed to open browser: {}", e);
                }
            });
        }
    }

    server::start_server(app_state).await?;

    Ok(())
}
