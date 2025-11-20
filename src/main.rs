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
    let console_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(console_layer)
        .init();

    // === Log Session Start ===
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    info!("---");
    info!("🚀 Старт сессии: PhotoMap Processor v{}", VERSION);
    info!(
        "🕒 Время запуска: {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    info!("---");

    // Register HEIC/HEIF decoder
    register_all_decoding_hooks();

    // Ensure single instance - kill existing processes
    process_manager::ensure_single_instance()?;

    // Initialize database
    info!("🗄️  Initializing database...");
    let db = Database::new().with_context(|| "Failed to initialize database")?;
    info!("✅ Database initialized successfully");
    info!("   📂 Database opened from: {}", Database::database_path());

    // Don't process photos here anymore - handled later with settings

    info!(
        "   📊 {} photos with GPS data in database",
        db.get_photos_count()?
    );
    info!("   🚀 Starting HTTP server for on-demand marker generation");

    // Start HTTP server
    let (event_sender, _event_receiver) = tokio::sync::broadcast::channel(100);
    let (shutdown_sender, _shutdown_receiver) = tokio::sync::broadcast::channel(1);

    let settings = Arc::new(Mutex::new(Settings::load()?));
    info!("   ⚙️  Config file loaded from: {}", Settings::config_path().display());

    // Process photos from last_folder if available
    {
        let settings_guard = settings.lock().unwrap();
        if let Some(ref folder_path) = settings_guard.last_folder {
            let photos_path = Path::new(folder_path);
            if photos_path.exists() {
                info!("🚀 Processing photos from saved folder: {}", folder_path);
                processing::process_photos_into_database(&db, photos_path)?;
            } else {
                warn!("⚠️  Saved folder not found: {}", folder_path);
                warn!("   Clearing database to remove stale data...");
                db.clear_all_photos()?;
                warn!("   Please select a folder using the web interface");
            }
        } else {
            warn!("⚠️  No saved folder found");
            warn!("   Clearing database to remove stale data...");
            db.clear_all_photos()?;
            warn!("   Please select a folder using the web interface");
        }
    } // Release the lock

    let app_state = AppState {
        db,
        settings,
        event_sender,
        shutdown_sender,
    };

    server::start_server(app_state).await?;

    Ok(())
}
