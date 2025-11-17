use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{header, StatusCode},
    response::{Html, Json, Response, Sse},
    routing::{get, post},
    Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::convert::Infallible;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::broadcast;
use tokio_stream::Stream;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

use crate::database::{Database, ImageMetadata};
use crate::image_processing::{create_scaled_image_in_memory, convert_heic_to_jpeg, ImageType};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/"]
struct Asset;
use crate::settings::Settings;
use std::sync::Arc;
use tokio::sync::mpsc;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};

// SSE Event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingEvent {
    pub event_type: String,
    pub data: ProcessingData,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessingData {
    pub total_files: Option<usize>,
    pub processed: Option<usize>,
    pub gps_found: Option<usize>,
    pub no_gps: Option<usize>,
    pub heic_files: Option<usize>,
    pub skipped: Option<usize>,
    pub current_file: Option<String>,
    pub speed: Option<f64>,
    pub eta: Option<String>,
    pub message: Option<String>,
    pub phase: Option<String>,
}

// Application state for sharing database and settings
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub settings: Arc<Mutex<Settings>>,
    pub event_sender: broadcast::Sender<ProcessingEvent>,
}

// HTTP API Handlers
async fn get_all_photos(State(state): State<AppState>) -> Result<Json<Vec<ImageMetadata>>, StatusCode> {
    let photos = state.db.get_all_photos()
        .map_err(|e| {
            eprintln!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let api_photos: Vec<ImageMetadata> = photos.into_iter().map(|photo| {
        let (url, fallback_url) = if photo.is_heic {
            // Для HEIC файлов основной URL - это конвертированный JPG
            let jpg_url = format!("/convert-heic?filename={}", photo.relative_path);
            (jpg_url.clone(), jpg_url)
        } else {
            let photo_url = format!("/photos/{}", photo.relative_path);
            (photo_url.clone(), photo_url)
        };

        ImageMetadata {
            filename: photo.filename.clone(),
            relative_path: photo.relative_path.clone(),
            url,
            fallback_url,
            marker_icon: format!("/api/marker/{}", photo.relative_path),
            lat: photo.lat,
            lng: photo.lng,
            datetime: photo.datetime,
            file_path: photo.file_path.clone(),
            is_heic: photo.is_heic,
        }
    }).collect();

    Ok(Json(api_photos))
}

/// Универсальная функция для обработки изображений (маркеры или превью)
async fn serve_processed_image(
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
    image_type: ImageType,
) -> Result<Response, StatusCode> {
    // Get photo file path from database
    let photos = state.db.get_all_photos()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let photo = photos.into_iter()
        .find(|p| p.relative_path == filename || p.filename == filename)
        .ok_or(StatusCode::NOT_FOUND)?;

    // For HEIC files, redirect to converted JPEG with proper size parameter
    if photo.is_heic {
        // Redirect to the converted HEIC image (served as JPEG)
        let size_param = image_type.name();
        let redirect_url = format!("/convert-heic?filename={}&size={}", filename, size_param);
        return Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, redirect_url)
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body("Redirecting to converted image".into())
            .unwrap());
    }

    // Generate image on-demand for non-HEIC files
    let png_data = create_scaled_image_in_memory(std::path::Path::new(&photo.file_path), image_type)
        .map_err(|e| {
            eprintln!("Failed to create {:?} for {}: {}", image_type, filename, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(png_data.into())
        .unwrap())
}

/// Обработчик для маркеров изображений (40x40px)
async fn get_marker_image(
    state: State<AppState>,
    filename: AxumPath<String>
) -> Result<Response, StatusCode> {
    serve_processed_image(state, filename, ImageType::Marker).await
}

/// Обработчик для превью изображений (60x60px)
async fn get_thumbnail_image(
    state: State<AppState>,
    filename: AxumPath<String>
) -> Result<Response, StatusCode> {
    serve_processed_image(state, filename, ImageType::Thumbnail).await
}

async fn convert_heic(
    State(state): State<AppState>,
    Query(query_params): Query<HashMap<String, String>>
) -> Result<Response, StatusCode> {
    let filename = query_params.get("filename").ok_or(StatusCode::BAD_REQUEST)?;
    let default_size = "full".to_string();
    let size_param = query_params.get("size").unwrap_or(&default_size);

    // Get full file path from database
    let photos = state.db.get_all_photos()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let photo = photos.into_iter()
        .find(|p| p.relative_path == *filename)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Convert HEIC to JPEG using our image processing module
    let jpeg_data = convert_heic_to_jpeg(&photo, size_param)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/jpeg")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(jpeg_data.into())
        .unwrap())
}

async fn serve_photo(
    State(state): State<AppState>,
    AxumPath(filepath): AxumPath<String>,
) -> Result<Response, StatusCode> {
    let base_dir = {
        let settings = state.settings.lock().unwrap();
        settings.last_folder.clone().unwrap_or_default()
    };

    let path = std::path::Path::new(&base_dir).join(&filepath);

    if !path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    let content_type = mime_guess::from_path(&path).first_or_octet_stream();

    match std::fs::read(&path) {
        Ok(data) => Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type.as_ref())
            .body(data.into())
            .unwrap()),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}



// API endpoint to get current settings
async fn get_settings(State(state): State<AppState>) -> Result<Json<Settings>, StatusCode> {
    let settings = state.settings.lock().unwrap();
    Ok(Json((*settings).clone()))
}

// API endpoint to set a folder path (receives path from browser's native dialog)
async fn set_folder(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>
) -> Result<Json<serde_json::Value>, StatusCode> {
    println!("🔍 Setting folder from browser dialog");

    // Extract folder_path from payload
    let raw_folder_path = match payload.get("folder_path").and_then(|v| v.as_str()) {
        Some(path) => path,
        None => {
            println!("❌ No folder_path provided");
            let response = serde_json::json!({
                "status": "error",
                "message": "No folder_path provided"
            });
            return Ok(Json(response));
        }
    };

    // Use the path as provided by the user
    let folder_path = raw_folder_path.to_string();
    let full_path = folder_path.clone();

    let mut settings = state.settings.lock().unwrap();
    settings.last_folder = Some(full_path.clone());

    // Save to INI file
    if let Err(e) = settings.save() {
        eprintln!("Failed to save settings: {}", e);
        // Continue without saving for now
    }

    println!("✅ Folder set: {} -> {}", folder_path, full_path);

    let response = serde_json::json!({
        "status": "success",
        "folder_path": full_path,
        "message": "Folder set successfully"
    });

    Ok(Json(response))
}

// API endpoint to update settings
async fn update_settings(
    State(state): State<AppState>,
    Json(new_settings): Json<Settings>
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut settings = state.settings.lock().unwrap();

    // Update settings
    *settings = new_settings.clone();

    // Save to disk
    if let Err(e) = settings.save() {
        eprintln!("Failed to save settings: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let response = serde_json::json!({
        "status": "success",
        "message": "Settings updated successfully"
    });

    Ok(Json(response))
}

// API endpoint to clear database and reprocess from selected folder
async fn reprocess_photos(
    State(state): State<AppState>
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get photos directory from settings
    let photos_dir = {
        let settings = state.settings.lock().unwrap();
        if let Some(ref folder) = settings.last_folder {
            std::path::Path::new(folder).to_path_buf()
        } else {
            // Default directory
            std::path::Path::new("/Users/dmitriiromanov/claude/photomap/photos").to_path_buf()
        }
    };

    let photos_dir_str = photos_dir.to_string_lossy().to_string();

    // Clear the database
    if let Err(e) = state.db.clear_all_photos() {
        eprintln!("Failed to clear database: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Clone the sender for the async task
    let event_sender = state.event_sender.clone();
    let db = state.db.clone();

    // Start processing in background task
    tokio::spawn(async move {
        // Use the synchronous processing function
        if let Err(e) = crate::process_photos_into_database(&db, &photos_dir) {
            eprintln!("Processing error: {}", e);

            // Send error event
            let error_event = ProcessingEvent {
                event_type: "processing_error".to_string(),
                data: ProcessingData {
                    message: Some(format!("Processing failed: {}", e)),
                    phase: Some("error".to_string()),
                    ..Default::default()
                },
            };
            let _ = event_sender.send(error_event);
        }
    });

    let response = serde_json::json!({
        "status": "started",
        "message": "Database cleared and photo processing started",
        "photos_directory": photos_dir_str
    });

    Ok(Json(response))
}

// API endpoint to start photo processing with SSE updates
async fn start_processing(
    State(state): State<AppState>
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Clone the sender for the async task
    let event_sender = state.event_sender.clone();
    let db = state.db.clone();

    // Get photos directory from settings
    let photos_dir = {
        let settings = state.settings.lock().unwrap();
        if let Some(ref folder) = settings.last_folder {
            std::path::Path::new(folder).to_path_buf()
        } else {
            // Default directory
            std::path::Path::new("/Users/dmitriiromanov/claude/photomap/photos").to_path_buf()
        }
    };

    // Start processing in background task
    tokio::spawn(async move {
        // Use the new processing function that works with selected directory
        match crate::process_photos_from_directory(&db, &photos_dir) {
            Ok((total_files, processed_count, gps_count, no_gps_count, heic_count)) => {
                // Send completion event
                let completion_event = ProcessingEvent {
                    event_type: "processing_complete".to_string(),
                    data: ProcessingData {
                        total_files: Some(total_files),
                        processed: Some(processed_count),
                        gps_found: Some(gps_count),
                        no_gps: Some(no_gps_count),
                        heic_files: Some(heic_count),
                        skipped: Some(total_files - processed_count),
                        message: Some(format!("Обработка завершена! Обработано {} фотографий из {}", processed_count, total_files)),
                        phase: Some("completed".to_string()),
                        ..Default::default()
                    },
                };
                let _ = event_sender.send(completion_event);
            }
            Err(e) => {
                eprintln!("Processing error: {}", e);

                // Send error event
                let error_event = ProcessingEvent {
                    event_type: "processing_error".to_string(),
                    data: ProcessingData {
                        message: Some(format!("Processing failed: {}", e)),
                        phase: Some("error".to_string()),
                        ..Default::default()
                    },
                };
                let _ = event_sender.send(error_event);
            }
        }
    });

    let response = serde_json::json!({
        "status": "started",
        "message": "Photo processing started with real-time updates"
    });

    Ok(Json(response))
}

// SSE endpoint for real-time processing updates
async fn processing_events_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let (tx, rx) = mpsc::channel(100);

    // Subscribe to the main event sender
    let mut event_receiver = state.event_sender.subscribe();

    // Forward events from main sender to SSE stream
    tokio::spawn(async move {
        loop {
            tokio::select! {
                event = event_receiver.recv() => {
                    match event {
                        Ok(processing_event) => {
                            let sse_event = SseEvent::default()
                                .json_data(&processing_event)
                                .unwrap_or_else(|_| SseEvent::default().data("Error serializing event"));

                            if tx.send(Ok(sse_event)).await.is_err() {
                                break; // Client disconnected
                            }
                        }
                        Err(_) => break, // Channel closed
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    // Send periodic heartbeat
                    let heartbeat = ProcessingEvent {
                        event_type: "heartbeat".to_string(),
                        data: ProcessingData {
                            message: Some("SSE connection alive".to_string()),
                            ..Default::default()
                        },
                    };

                    let sse_event = SseEvent::default()
                        .json_data(&heartbeat)
                        .unwrap_or_else(|_| SseEvent::default().data("Error serializing heartbeat"));

                    if tx.send(Ok(sse_event)).await.is_err() {
                        break; // Client disconnected
                    }
                }
            }
        }
    });

    let stream = ReceiverStream::new(rx);

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive-message".to_string()),
    )
}

// Helper struct for SSE events
use axum::response::sse::Event as SseEvent;

// Create the main application router
async fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_html))
        .route("/style.css", get(style_css))
        .route("/script.js", get(script_js))
        .route("/api/photos", get(get_all_photos))
        .route("/api/marker/*filename", get(get_marker_image))
        .route("/api/thumbnail/*filename", get(get_thumbnail_image))
        .route("/convert-heic", get(convert_heic))
        .route("/api/settings", get(get_settings))
        .route("/api/set-folder", post(set_folder))
        .route("/api/settings", axum::routing::post(update_settings))
        .route("/api/events", get(processing_events_stream))
        .route("/api/process", axum::routing::post(start_processing))
        .route("/api/reprocess", axum::routing::post(reprocess_photos))
        .route("/photos/*filepath", get(serve_photo))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
        )
        .with_state(state)
}

pub async fn start_server(state: AppState) -> Result<()> {
    start_server_with_port(state, 3001).await
}

async fn index_html() -> Html<Vec<u8>> {
    Html(Asset::get("index.html").unwrap().data.into_owned())
}

async fn style_css() -> Response {
    let content = Asset::get("style.css").unwrap().data;
    Response::builder()
        .header(header::CONTENT_TYPE, "text/css")
        .body(content.into_owned().into())
        .unwrap()
}

async fn script_js() -> Response {
    let content = Asset::get("script.js").unwrap().data;
    Response::builder()
        .header(header::CONTENT_TYPE, "application/javascript")
        .body(content.into_owned().into())
        .unwrap()
}

async fn start_server_with_port(state: AppState, port: u16) -> Result<()> {
    let app = create_app(state).await;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;

    println!("   ✅ HTTP server started successfully");
    println!("   🗺️  API endpoints available:");
    println!("      - GET /api/photos - List all photos with GPS data");
    println!("      - GET /api/marker/<filename> - Generate 40x40px marker icon");
    println!("      - GET /api/thumbnail/<filename> - Generate 60x60px thumbnail");
    println!("      - GET /convert-heic?filename=<name> - Convert HEIC to JPEG");
    println!("      - GET/POST /api/settings - Load/save application settings");
    println!("      - GET /api/events - Real-time processing updates (SSE)");
    println!("      - POST /api/process - Start photo processing with SSE updates");
    println!("   🎯 Features: 700px popups + HEIC support + folder selection + real-time processing");

    axum::serve(listener, app).await?;
    Ok(())
}