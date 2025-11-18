use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

pub mod events;
pub mod handlers;
pub mod state;

use self::state::AppState;
use handlers::{
    convert_heic, get_all_photos, get_marker_image, get_popup_image, get_settings,
    get_thumbnail_image, index_html, initiate_processing, processing_events_stream,
    reprocess_photos, script_js, serve_photo, set_folder, style_css, update_settings,
};

// Create the main application router
async fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_html))
        .route("/style.css", get(style_css))
        .route("/script.js", get(script_js))
        .route("/api/photos", get(get_all_photos))
        .route("/api/marker/*filename", get(get_marker_image))
        .route("/api/thumbnail/*filename", get(get_thumbnail_image))
        .route("/api/popup/*filename", get(get_popup_image))
        .route("/convert-heic", get(convert_heic))
        .route("/api/settings", get(get_settings))
        .route("/api/set-folder", post(set_folder))
        .route("/api/settings", axum::routing::post(update_settings))
        .route("/api/events", get(processing_events_stream))
        .route("/api/initiate-processing", post(initiate_processing))
        .route("/api/reprocess", axum::routing::post(reprocess_photos))
        .route("/photos/*filepath", get(serve_photo))
        .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
        .with_state(state)
}

pub async fn start_server(state: AppState) -> Result<()> {
    start_server_with_port(state, 3001).await
}

async fn start_server_with_port(state: AppState, port: u16) -> Result<()> {
    let app = create_app(state).await;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;

    println!("   ✅ HTTP server started successfully at http://127.0.0.1:{}", port);

    axum::serve(listener, app).await?;
    Ok(())
}
