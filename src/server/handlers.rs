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
use crate::processing::{process_photos_from_directory, process_photos_with_stats};
use crate::geocoding;

/// Simple MIME type detection based on file extension
fn get_mime_type(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()) {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("heic") | Some("heif") => "image/heic",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}
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
                location: geocoding::get_location_name(photo.lat, photo.lng),
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

/// Handler for image thumbnails (120x120px for map markers)
pub async fn get_thumbnail_image(
    state: State<AppState>,
    filename: AxumPath<String>,
) -> Result<Response, StatusCode> {
    serve_processed_image(state, filename, ImageType::Thumbnail).await
}

/// Handler for gallery images (240x240px for gallery modal)
pub async fn get_gallery_image(
    state: State<AppState>,
    filename: AxumPath<String>,
) -> Result<Response, StatusCode> {
    serve_processed_image(state, filename, ImageType::Gallery).await
}

/// Handler for popup images (1400px)
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
        settings.folders[0].clone().unwrap_or_default()
    };

    let path = std::path::Path::new(&base_dir).join(&filepath);

    if !path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    let content_type = get_mime_type(&path);

    match std::fs::read(&path) {
        Ok(data) => Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
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

// API endpoint to set folder path(s) - supports both single and multiple folders
pub async fn set_folder(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    println!("🔍 Setting folder(s) from browser dialog");

    // Try to extract folder_paths array first, then fallback to single folder_path
    let folder_paths = if let Some(paths_array) = payload.get("folder_paths").and_then(|v| v.as_array()) {
        // Multiple folders
        paths_array
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<String>>()
    } else if let Some(single_path) = payload.get("folder_path").and_then(|v| v.as_str()) {
        // Single folder (backward compatibility)
        vec![single_path.to_string()]
    } else {
        println!("❌ No folder_path or folder_paths provided");
        let response = serde_json::json!({
            "status": "error",
            "message": "No folder_path or folder_paths provided"
        });
        return Ok(Json(response));
    };

    if folder_paths.is_empty() {
        println!("❌ Empty folder list provided");
        let response = serde_json::json!({
            "status": "error",
            "message": "Empty folder list"
        });
        return Ok(Json(response));
    }

    // Limit to 5 folders
    let folders_to_store: Vec<String> = folder_paths.into_iter().take(5).collect();

    // Validate that all folders exist
    for folder_path in &folders_to_store {
        if !std::path::Path::new(folder_path).exists() {
            println!("❌ Folder does not exist: {}", folder_path);
            let response = serde_json::json!({
                "status": "error",
                "message": format!("Folder does not exist: {}", folder_path)
            });
            return Ok(Json(response));
        }
    }

    // Store all folders in settings
    let mut settings = state.settings.lock().unwrap();
    
    // Clear all slots first
    for i in 0..5 {
        settings.folders[i] = None;
    }
    
    // Store provided folders
    for (i, folder_path) in folders_to_store.iter().enumerate() {
        settings.folders[i] = Some(folder_path.clone());
        println!("  {}. {}", i + 1, folder_path);
    }

    // Save to INI file
    if let Err(e) = settings.save() {
        eprintln!("Failed to save settings: {}", e);
    }

    println!("✅ Stored {} folder(s)", folders_to_store.len());

    let response = serde_json::json!({
        "status": "success",
        "folder_paths": folders_to_store,
        "count": folders_to_store.len(),
        "message": if folders_to_store.len() > 1 {
            format!("{} folders set", folders_to_store.len())
        } else {
            "Folder set successfully".to_string()
        }
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
    // Get folders from settings
    let folders_to_process = {
        let settings = state.settings.lock().unwrap();
        settings.folders
            .iter()
            .filter_map(|f| f.as_ref().map(|s| std::path::Path::new(s).to_path_buf()))
            .collect::<Vec<_>>()
    };
    
    if folders_to_process.is_empty() {
        let response = serde_json::json!({
            "status": "error",
            "message": "No folders configured"
        });
        return Ok(Json(response));
    }
    
    // Clear the database once before processing all folders
    if let Err(e) = state.db.clear_all_photos() {
        eprintln!("Failed to clear database: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Clone the sender for the async task
    let event_sender = state.event_sender.clone();
    let db = state.db.clone();
    let folders_clone = folders_to_process.clone();

    // Start processing in background task
    tokio::spawn(async move {
        let mut total_stats = (0usize, 0usize, 0usize, 0usize, 0usize);
        
        for photos_dir in &folders_clone {
            if !photos_dir.exists() {
                eprintln!("⚠️  Folder not found: {}", photos_dir.display());
                continue;
            }
            
            // Use process_photos_with_stats with clear_database=false (DB already cleared once)
            match process_photos_with_stats(&db, photos_dir, false, false) {
                Ok((total_files, processed_count, gps_count, no_gps_count, heic_count)) => {
                    // Aggregate statistics
                    total_stats.0 += total_files;
                    total_stats.1 += processed_count;
                    total_stats.2 += gps_count;
                    total_stats.3 += no_gps_count;
                    total_stats.4 += heic_count;
                }
                Err(e) => {
                    eprintln!("Processing error for {}: {}", photos_dir.display(), e);
                    
                    // Send error event
                    let error_event = ProcessingEvent {
                        event_type: "processing_error".to_string(),
                        data: ProcessingData {
                            message: Some(format!("Processing failed for {}: {}", photos_dir.display(), e)),
                            phase: Some("error".to_string()),
                            ..Default::default()
                        },
                    };
                    let _ = event_sender.send(error_event);
                }
            }
        }
        
        // Send completion event with aggregated stats
        let completion_event = ProcessingEvent {
            event_type: "processing_complete".to_string(),
            data: ProcessingData {
                total_files: Some(total_stats.0),
                processed: Some(total_stats.1),
                gps_found: Some(total_stats.2),
                no_gps: Some(total_stats.3),
                heic_files: Some(total_stats.4),
                skipped: Some(total_stats.0 - total_stats.1),
                message: Some(format!(
                    "Processing finished! Processed {} photos from {} folder(s)",
                    total_stats.1, folders_clone.len()
                )),
                phase: Some("completed".to_string()),
                ..Default::default()
            },
        };
        let _ = event_sender.send(completion_event);
    });

    let response = serde_json::json!({
        "status": "started",
        "message": format!("Database cleared and processing {} folder(s)", folders_to_process.len()),
        "count": folders_to_process.len()
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

    // Get folders from settings
    let folders_to_process = {
        let settings = state.settings.lock().unwrap();
        settings.folders
            .iter()
            .filter_map(|f| f.as_ref().map(|s| std::path::Path::new(s).to_path_buf()))
            .collect::<Vec<_>>()
    };

    if folders_to_process.is_empty() {
        tokio::spawn(async move {
            let error_event = ProcessingEvent {
                event_type: "processing_error".to_string(),
                data: ProcessingData {
                    message: Some("No folders configured".to_string()),
                    phase: Some("error".to_string()),
                    ..Default::default()
                },
            };
            let _ = event_sender.send(error_event);
        });
        
        let response = serde_json::json!({
            "status": "error",
            "message": "No folders configured"
        });
        return Ok(Json(response));
    }

    let folders_clone = folders_to_process.clone();
    
    // Start processing in background task for all folders
    tokio::spawn(async move {
        let mut total_stats = (0usize, 0usize, 0usize, 0usize, 0usize);
        
        for photos_dir in &folders_clone {
            if !photos_dir.exists() {
                eprintln!("⚠️  Folder not found: {}", photos_dir.display());
                
                let error_event = ProcessingEvent {
                    event_type: "processing_error".to_string(),
                    data: ProcessingData {
                        message: Some(format!("Folder not found: {}", photos_dir.display())),
                        phase: Some("error".to_string()),
                        ..Default::default()
                    },
                };
                let _ = event_sender.send(error_event);
                continue;
            }
            
            let result = process_photos_from_directory(&db, photos_dir);

            match result {
                Ok((total_files, processed_count, gps_count, no_gps_count, heic_count)) => {
                    // Aggregate statistics
                    total_stats.0 += total_files;
                    total_stats.1 += processed_count;
                    total_stats.2 += gps_count;
                    total_stats.3 += no_gps_count;
                    total_stats.4 += heic_count;
                }
                Err(e) => {
                    eprintln!("Processing error for {}: {}", photos_dir.display(), e);
                    let error_event = ProcessingEvent {
                        event_type: "processing_error".to_string(),
                        data: ProcessingData {
                            message: Some(format!("Processing failed for {}: {}", photos_dir.display(), e)),
                            phase: Some("error".to_string()),
                            ..Default::default()
                        },
                    };
                    let _ = event_sender.send(error_event);
                }
            }
        }
        
        // Send completion event with aggregated stats
        let completion_event = ProcessingEvent {
            event_type: "processing_complete".to_string(),
            data: ProcessingData {
                total_files: Some(total_stats.0),
                processed: Some(total_stats.1),
                gps_found: Some(total_stats.2),
                no_gps: Some(total_stats.3),
                heic_files: Some(total_stats.4),
                skipped: Some(total_stats.0 - total_stats.1),
                message: Some(format!(
                    "Processing finished! Processed {} photos from {} folder(s)",
                    total_stats.1, folders_clone.len()
                )),
                phase: Some("completed".to_string()),
                ..Default::default()
            },
        };
        let _ = event_sender.send(completion_event);
    });

    let response = serde_json::json!({
        "status": "started",
        "message": format!("Processing {} folder(s)", folders_to_process.len()),
        "count": folders_to_process.len()
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

// API endpoint to open native folder selection dialog (supports multiple folders)
pub async fn select_folder_dialog(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    println!("🔍 Opening native folder selection dialog...");

    // Call the native folder picker (supports multiple on macOS/Linux, sequential on Windows)
    let folder_paths = tokio::task::spawn_blocking(|| {
        crate::utils::select_folders_native()
    }).await.map_err(|e| {
        eprintln!("Task join error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if !folder_paths.is_empty() {
        println!("✅ Selected {} folder(s)", folder_paths.len());
        for (i, path) in folder_paths.iter().enumerate() {
            println!("   {}. {}", i + 1, path);
        }
        
        let response = serde_json::json!({
            "status": "success",
            "folder_paths": folder_paths,  // Array instead of single path
            "count": folder_paths.len(),
            "message": if folder_paths.len() > 1 {
                format!("{} folders selected", folder_paths.len())
            } else {
                "Folder selected".to_string()
            }
        });
        Ok(Json(response))
    } else {
        println!("❌ Folder selection cancelled");
        let response = serde_json::json!({
            "status": "cancelled",
            "message": "Folder selection cancelled"
        });
        Ok(Json(response))
    }
}

/// Reveal photo in system file manager
pub async fn reveal_file(
    Json(file_path): Json<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use std::process::Command;
    
    println!("📁 Reveal in explorer: {}", file_path);
    
    let result = {
        #[cfg(target_os = "windows")]
        {
            // Ensure backslashes for Windows path
            let clean_path = file_path.replace("/", "\\");
            
            // Use "cmd /C start" to launch explorer. This often helps with bringing the window 
            // to the foreground compared to spawning explorer directly.
            // Syntax: start ["title"] [program] [args...]
            // We pass an empty string for title to avoid "explorer" being interpreted as the title.
            Command::new("cmd")
                .args(["/C", "start", "", "explorer", "/select,", &clean_path])
                .spawn()
        }
        
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg("-R")
                .arg(&file_path)
                .spawn()
        }
        
        #[cfg(target_os = "linux")]
        {
            // Try nautilus first (GNOME), fallback to xdg-open
            Command::new("nautilus")
                .arg("--select")
                .arg(&file_path)
                .spawn()
                .or_else(|_| {
                    // Fallback: open containing directory
                    use std::path::Path;
                    let parent = Path::new(&file_path).parent()
                        .and_then(|p| p.to_str())
                        .unwrap_or(&file_path);
                    Command::new("xdg-open")
                        .arg(parent)
                        .spawn()
                })
        }
    };
    
    match result {
        Ok(_) => {
            println!("✅ Opened file manager");
            Ok(Json(serde_json::json!({
                "status": "success",
                "message": "File revealed in explorer"
            })))
        }
        Err(e) => {
            eprintln!("❌ Failed to open file manager: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
