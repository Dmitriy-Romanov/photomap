use anyhow::{Context, Result};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

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
    // === Setup Logging ===
    struct CustomTimer;

    impl tracing_subscriber::fmt::time::FormatTime for CustomTimer {
        fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
            let now = chrono::Local::now();
            write!(w, "{}", now.format("%d %H:%M:%S"))
        }
    }

    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_timer(CustomTimer);
    
    // Set default log level to INFO, but allow overriding via RUST_LOG env var
    // This prevents verbose logs from dependencies like 'ignore' unless explicitly requested
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(console_layer)
        .with(env_filter)
        .init();

    // === Log Session Start ===
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    info!("---");
    info!("🚀 Session start: PhotoMap Processor v{}", VERSION);
    info!(
        "🕒 Launch time: {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    info!("---");

    // Register HEIC/HEIF decoder
    register_all_decoding_hooks();

    // Ensure single instance - kill existing processes
    process_manager::ensure_single_instance()?;

    // Initialize database
    info!("🗄️  Initializing database (In-Memory)...");
    let db = Database::new().with_context(|| "Failed to initialize database")?;
    info!("✅ Database initialized successfully");

    // Initialize Reverse Geocoder (Lazy load in background)
    info!("🌍 Initializing Reverse Geocoder...");
    std::thread::spawn(|| {
        geocoding::ReverseGeocoder::init();
    });

    // Don't process photos here anymore - handled later with settings

    info!("   🚀 Starting HTTP server for on-demand marker generation");

    // Start HTTP server
    let (event_sender, _event_receiver) = tokio::sync::broadcast::channel(100);
    let (shutdown_sender, _shutdown_receiver) = tokio::sync::broadcast::channel(1);

    let settings = Arc::new(Mutex::new(Settings::load()?));
    info!("   ⚙️  Config file loaded from: {}", Settings::config_path().display());

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
                    warn!("⚠️  Failed to load cache: {}", e);
                    false
                }
            };
            
            if cache_loaded {
                let count = db.get_photos_count().unwrap_or(0);
                info!("✅ Loaded {} photos from cache (paths match)", count);
            } else {
                info!("🚀 Cache miss or mismatch. Processing {} folder(s)...", folder_paths.len());
                
                // Clear database once before processing all folders
                if let Err(e) = db.clear_all_photos() {
                    warn!("⚠️  Failed to clear database: {}", e);
                }
                
                for folder_path in &folder_paths {
                    let photos_path = Path::new(folder_path);
                    if !photos_path.exists() {
                        warn!("⚠️  Saved folder not found: {}", folder_path);
                        continue;
                    }
                    
                    info!("📂 Processing saved folder: {}", folder_path);
                    
                    // Process without clearing (DB already cleared once above)
                    match processing::process_photos_with_stats(&db, photos_path, false, false) {
                        Ok(_) => {},
                        Err(e) => warn!("⚠️  Error processing {}: {}", folder_path, e),
                    }
                }
                
                let count = db.get_photos_count().unwrap_or(0);
                info!("✅ Total photos in database: {}", count);
                
                // Save to cache
                if let Err(e) = db.save_to_disk(&folder_paths) {
                    warn!("⚠️  Failed to save cache: {}", e);
                } else {
                    info!("💾 Cache saved successfully");
                }
            }
        } else {
            info!("ℹ️  No saved folders found. Please select folders using the web interface");
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
            info!("   🌐 Opening browser at {}", url);
            
            // Spawn a task to open the browser after a short delay to ensure server is up
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if let Err(e) = utils::open_browser(url) {
                    warn!("Failed to open browser: {}", e);
                }
            });
        }
    }

    server::start_server(app_state).await?;

    Ok(())
}
