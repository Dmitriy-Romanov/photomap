use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{header, StatusCode},
    response::{Html, Json, Response, Sse},
};
use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

use crate::database::ImageMetadata;
use crate::image_processing::{convert_heic_to_jpeg, create_scaled_image_in_memory, ImageType};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/"]
struct Asset;
use crate::processing::{process_photos_from_directory, process_photos_into_database};
use crate::settings::Settings;
use tokio::sync::mpsc;

use super::events::{ProcessingData, ProcessingEvent};
use super::state::AppState;

// HTTP API Handlers
pub async fn get_all_photos(
    State(state): State<AppState>,
) -> Result<Json<Vec<ImageMetadata>>, StatusCode> {
    let photos = state.db.get_all_photos().map_err(|e| {
        eprintln!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let api_photos: Vec<ImageMetadata> = photos
        .into_iter()
        .map(|photo| {
            let (url, fallback_url) = if photo.is_heic {
                // For HEIC files, the main URL is the converted JPG
                let jpg_url = format!("/convert-heic?filename={}", photo.relative_path);
                (jpg_url.clone(), jpg_url)
            } else {
                let photo_url = format!("/api/popup/{}", photo.relative_path);
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
        })
        .collect();

    Ok(Json(api_photos))
}

/// Universal function for image processing (markers or thumbnails)
pub async fn serve_processed_image(
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
    image_type: ImageType,
) -> Result<Response, StatusCode> {
    // Get photo file path from database
    let photos = state
        .db
        .get_all_photos()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let photo = photos
        .into_iter()
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
    let jpeg_data =
        create_scaled_image_in_memory(std::path::Path::new(&photo.file_path), image_type).map_err(
            |e| {
                eprintln!("Failed to create {:?} for {}: {}", image_type, filename, e);
                StatusCode::INTERNAL_SERVER_ERROR
            },
        )?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/jpeg")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(jpeg_data.into())
        .unwrap())
}

/// Handler for image markers (40x40px)
pub async fn get_marker_image(
    state: State<AppState>,
    filename: AxumPath<String>,
) -> Result<Response, StatusCode> {
    serve_processed_image(state, filename, ImageType::Marker).await
}

/// Handler for image thumbnails (60x60px)
pub async fn get_thumbnail_image(
    state: State<AppState>,
    filename: AxumPath<String>,
) -> Result<Response, StatusCode> {
    serve_processed_image(state, filename, ImageType::Thumbnail).await
}

/// Handler for popup images (700px)
pub async fn get_popup_image(
    state: State<AppState>,
    filename: AxumPath<String>,
) -> Result<Response, StatusCode> {
    serve_processed_image(state, filename, ImageType::Popup).await
}

pub async fn convert_heic(
    State(state): State<AppState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Response, StatusCode> {
    let filename = query_params
        .get("filename")
        .ok_or(StatusCode::BAD_REQUEST)?;
    let default_size = "popup".to_string();
    let size_param = query_params.get("size").unwrap_or(&default_size);

    // Get full file path from database
    let photos = state
        .db
        .get_all_photos()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let photo = photos
        .into_iter()
        .find(|p| p.relative_path == *filename)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Convert HEIC to JPEG using our image processing module
    let jpeg_data =
        convert_heic_to_jpeg(&photo, size_param).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/jpeg")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(jpeg_data.into())
        .unwrap())
}

pub async fn serve_photo(
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
pub async fn get_settings(State(state): State<AppState>) -> Result<Json<Settings>, StatusCode> {
    let settings = state.settings.lock().unwrap();
    Ok(Json((*settings).clone()))
}

// API endpoint to set a folder path (receives path from browser's native dialog)
pub async fn set_folder(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
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

    // Validate that the folder exists
    if !std::path::Path::new(&full_path).exists() {
        println!("❌ Folder does not exist: {}", full_path);
        let response = serde_json::json!({
            "status": "error",
            "message": format!("Folder does not exist: {}", full_path)
        });
        return Ok(Json(response));
    }

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
pub async fn update_settings(
    State(state): State<AppState>,
    Json(new_settings): Json<Settings>,
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
pub async fn reprocess_photos(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get photos directory from settings
    let photos_dir = {
        let settings = state.settings.lock().unwrap();
        if let Some(ref folder) = settings.last_folder {
            std::path::Path::new(folder).to_path_buf()
        } else {
            // No folder configured
            let response = serde_json::json!({
                "status": "error",
                "message": "No folder configured"
            });
            return Ok(Json(response));
        }
    };

    // Check if directory exists
    if !photos_dir.exists() {
        let response = serde_json::json!({
            "status": "error",
            "message": format!("Photos directory not found: {}", photos_dir.display())
        });
        return Ok(Json(response));
    }

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
        if let Err(e) = process_photos_into_database(&db, &photos_dir) {
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

// API endpoint to start photo processing
pub async fn initiate_processing(
    State(state): State<AppState>,
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
            // No folder configured - send error event
            tokio::spawn(async move {
                let error_event = ProcessingEvent {
                    event_type: "processing_error".to_string(),
                    data: ProcessingData {
                        message: Some("No folder configured".to_string()),
                        phase: Some("error".to_string()),
                        ..Default::default()
                    },
                };
                let _ = event_sender.send(error_event);
            });
            
            let response = serde_json::json!({
                "status": "error",
                "message": "No folder configured"
            });
            return Ok(Json(response));
        }
    };

    // Check if directory exists
    if !photos_dir.exists() {
        // Clear the database so we don't show old photos
        if let Err(e) = state.db.clear_all_photos() {
            eprintln!("Failed to clear database: {}", e);
        }

        tokio::spawn(async move {
            let error_event = ProcessingEvent {
                event_type: "processing_error".to_string(),
                data: ProcessingData {
                    message: Some(format!("Photos directory not found: {}", photos_dir.display())),
                    phase: Some("error".to_string()),
                    ..Default::default()
                },
            };
            let _ = event_sender.send(error_event);
        });

        let response = serde_json::json!({
            "status": "error",
            "message": "Photos directory not found"
        });
        return Ok(Json(response));
    }

    // Start processing in background task
    tokio::spawn(async move {
        let result = process_photos_from_directory(&db, &photos_dir);

        let completion_event = match result {
            Ok((total_files, processed_count, gps_count, no_gps_count, heic_count)) => {
                ProcessingEvent {
                    event_type: "processing_complete".to_string(),
                    data: ProcessingData {
                        total_files: Some(total_files),
                        processed: Some(processed_count),
                        gps_found: Some(gps_count),
                        no_gps: Some(no_gps_count),
                        heic_files: Some(heic_count),
                        skipped: Some(total_files - processed_count),
                        message: Some(format!(
                            "Processing finished! Processed {} photos out of {}",
                            processed_count, total_files
                        )),
                        phase: Some("completed".to_string()),
                        ..Default::default()
                    },
                }
            }
            Err(e) => {
                eprintln!("Processing error: {}", e);
                ProcessingEvent {
                    event_type: "processing_error".to_string(),
                    data: ProcessingData {
                        message: Some(format!("Processing failed: {}", e)),
                        phase: Some("error".to_string()),
                        ..Default::default()
                    },
                }
            }
        };
        let _ = event_sender.send(completion_event);
    });

    let response = serde_json::json!({
        "status": "started",
        "message": "Photo processing initiated"
    });

    Ok(Json(response))
}

// SSE endpoint for real-time processing updates
pub async fn processing_events_stream(
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
            .text("keepalive-message"),
    )
}

// Helper struct for SSE events
use axum::response::sse::Event as SseEvent;

pub async fn index_html() -> Html<Vec<u8>> {
    Html(Asset::get("index.html").unwrap().data.into_owned())
}

pub async fn style_css() -> Response {
    let content = Asset::get("style.css").unwrap().data;
    Response::builder()
        .header(header::CONTENT_TYPE, "text/css")
        .body(content.into_owned().into())
        .unwrap()
}

pub async fn script_js() -> Response {
    let content = Asset::get("script.js").unwrap().data;
    Response::builder()
        .header(header::CONTENT_TYPE, "application/javascript")
        .body(content.into_owned().into())
        .unwrap()
}

// API endpoint to shut down the server
pub async fn shutdown_app(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    println!("🛑 Received shutdown request");

    // Send shutdown signal
    let _ = state.shutdown_sender.send(());

    let response = serde_json::json!({
        "status": "success",
        "message": "Server shutting down"
    });

    Ok(Json(response))
}
