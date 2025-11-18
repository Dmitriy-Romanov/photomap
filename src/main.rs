use anyhow::{Context, Result};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{info, warn, error};
use tracing_subscriber;

// Import modules
mod constants;
mod database;
mod folder_picker;
mod image_processing;
mod exif_parser;
mod processing;
mod settings;
pub mod server;
mod process_manager;
mod utils;

use database::Database;
use libheif_rs::integration::image::register_all_decoding_hooks;
use server::state::AppState;
use settings::Settings;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("🗺️  PhotoMap Processor v0.6.2 - Enhanced UI Edition starting...");

    // Register HEIC/HEIF decoder
    register_all_decoding_hooks();

    // Ensure single instance - kill existing processes
    process_manager::ensure_single_instance()?;

    info!("✅ Native HEIC/HEIF support enabled");

    // Initialize database
    info!("🗄️  Initializing database...");
    let db = Database::new()
        .with_context(|| "Failed to initialize database")?;
    info!("✅ Database initialized successfully");

    // Don't process photos here anymore - handled later with settings

    info!("\n🎉 Phase 3 implementation ready!");
    info!("   📊 {} photos with GPS data in database", db.get_photos_count()?);
    info!("   🚀 Starting HTTP server for on-demand marker generation");

    // Start HTTP server
    let (event_sender, _event_receiver) = tokio::sync::broadcast::channel(100);

    let settings = Arc::new(Mutex::new(Settings::load()?));

    // Process photos from last_folder if available
    {
        let settings_guard = settings.lock().unwrap();
        if let Some(ref folder_path) = settings_guard.last_folder {
            let photos_path = Path::new(folder_path);
            if photos_path.exists() {
                info!("\n🚀 Processing photos from saved folder: {}", folder_path);
                processing::process_photos_into_database(&db, photos_path)?;
            } else {
                warn!("\n⚠️  Saved folder not found: {}", folder_path);
                warn!("   Please select a folder using the web interface");
            }
        } else {
            warn!("\n⚠️  No saved folder found");
            warn!("   Please select a folder using the web interface");
        }
    } // Release the lock

    let app_state = AppState {
        db,
        settings,
        event_sender,
    };

    server::start_server(app_state).await?;

    Ok(())
}
