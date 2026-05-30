use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{header, StatusCode},
    response::{Html, Json, Response, Sse},
};
use futures_core::Stream;
use std::collections::HashMap;
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::database::ImageMetadata;
use crate::geocoding;
use crate::image_processing::{convert_heic_to_jpeg, create_scaled_image_in_memory, ImageType};
use crate::processing::{process_photos_from_directory, process_photos_with_stats};
use crate::settings::Settings;

use super::events::{ProcessingData, ProcessingEvent};
use super::state::AppState;

const INDEX_HTML: &[u8] = include_bytes!("../../frontend/index.html");
const STYLE_CSS: &[u8] = include_bytes!("../../frontend/style.css");
const SCRIPT_JS: &[u8] = include_bytes!("../../frontend/script.js");

struct ReceiverStream {
    rx: mpsc::Receiver<Result<axum::response::sse::Event, Infallible>>,
}

impl Stream for ReceiverStream {
    type Item = Result<axum::response::sse::Event, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

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

pub async fn get_all_photos(
    State(state): State<AppState>,
) -> Result<Json<Vec<ImageMetadata>>, StatusCode> {
    let photos = match tokio::task::spawn_blocking({
        let db = state.db.clone();
        move || db.get_all_photos()
    })
    .await
    {
        Ok(Ok(photos)) => photos,
        Ok(Err(e)) => {
            eprintln!("Database error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let api_photos: Vec<ImageMetadata> = photos
        .into_iter()
        .map(|photo| {
            let (url, fallback_url) = if photo.is_heic {
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

pub async fn serve_processed_image(
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
    image_type: ImageType,
) -> Result<Response, StatusCode> {
    let photo = state
        .db
        .get_photo_by_relative_path(&filename)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if photo.is_heic {
        let size_param = image_type.name();
        let redirect_url = format!("/convert-heic?filename={}&size={}", filename, size_param);
        return Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .header(header::LOCATION, redirect_url)
            .body("Redirecting to converted image".into())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
    }

    let jpeg_data = match tokio::task::spawn_blocking(move || {
        create_scaled_image_in_memory(std::path::Path::new(&photo.file_path), image_type)
    })
    .await
    {
        Ok(Ok(data)) => data,
        Ok(Err(e)) => {
            eprintln!("Image processing error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/jpeg")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(jpeg_data.into())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}

pub async fn get_marker_image(
    state: State<AppState>,
    filename: AxumPath<String>,
) -> Result<Response, StatusCode> {
    serve_processed_image(state, filename, ImageType::Marker).await
}

pub async fn get_thumbnail_image(
    state: State<AppState>,
    filename: AxumPath<String>,
) -> Result<Response, StatusCode> {
    serve_processed_image(state, filename, ImageType::Thumbnail).await
}

pub async fn get_gallery_image(
    state: State<AppState>,
    filename: AxumPath<String>,
) -> Result<Response, StatusCode> {
    serve_processed_image(state, filename, ImageType::Gallery).await
}

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
    let size_param = query_params
        .get("size")
        .cloned()
        .unwrap_or_else(|| "popup".to_string());

    let photo = state
        .db
        .get_photo_by_relative_path(filename)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let jpeg_data = match tokio::task::spawn_blocking(move || {
        convert_heic_to_jpeg(&photo, &size_param)
    })
    .await
    {
        Ok(Ok(data)) => data,
        Ok(Err(e)) => {
            eprintln!("HEIC conversion error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        Err(e) => {
            eprintln!("HEIC conversion task failed: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/jpeg")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(jpeg_data.into())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}

pub async fn serve_photo(
    State(state): State<AppState>,
    AxumPath(filepath): AxumPath<String>,
) -> Result<Response, StatusCode> {
    let photo = state
        .db
        .get_photo_by_relative_path(&filepath)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let path = std::path::Path::new(&photo.file_path);
    if !path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }
    let data = tokio::fs::read(path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let content_type = get_mime_type(path);
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(data.into())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}

pub async fn get_settings(State(state): State<AppState>) -> Result<Json<Settings>, StatusCode> {
    let settings = state.settings.lock().await;
    Ok(Json((*settings).clone()))
}

pub async fn set_folder(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let folder_paths =
        if let Some(paths_array) = payload.get("folder_paths").and_then(|v| v.as_array()) {
            paths_array
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        } else if let Some(single_path) = payload.get("folder_path").and_then(|v| v.as_str()) {
            vec![single_path.to_string()]
        } else {
            return Ok(Json(serde_json::json!({
                "status": "error",
                "message": "No folder_path or folder_paths provided"
            })));
        };

    if folder_paths.is_empty() {
        return Ok(Json(serde_json::json!({
            "status": "error",
            "message": "Empty folder list"
        })));
    }

    let folders_to_store: Vec<String> = folder_paths.into_iter().take(5).collect();

    for folder_path in &folders_to_store {
        if !std::path::Path::new(folder_path).exists() {
            return Ok(Json(serde_json::json!({
                "status": "error",
                "message": format!("Folder does not exist: {}", folder_path)
            })));
        }
    }

    let mut settings = state.settings.lock().await;
    for i in 0..5 {
        settings.folders[i] = None;
    }
    for (i, folder_path) in folders_to_store.iter().enumerate() {
        settings.folders[i] = Some(folder_path.clone());
    }

    if let Err(e) = settings.save() {
        eprintln!("Failed to save settings: {}", e);
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "folder_paths": folders_to_store,
        "count": folders_to_store.len(),
        "message": if folders_to_store.len() > 1 {
            format!("{} folders set", folders_to_store.len())
        } else {
            "Folder set successfully".to_string()
        }
    })))
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(new_settings): Json<Settings>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut settings = state.settings.lock().await;
    *settings = new_settings.clone();

    if let Err(e) = settings.save() {
        eprintln!("Failed to save settings: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Settings updated successfully"
    })))
}

pub async fn reprocess_photos(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let folders_to_process = {
        let settings = state.settings.lock().await;
        settings
            .folders
            .iter()
            .filter_map(|f| f.as_ref().map(|s| std::path::Path::new(s).to_path_buf()))
            .collect::<Vec<_>>()
    };

    if folders_to_process.is_empty() {
        return Ok(Json(serde_json::json!({
            "status": "error",
            "message": "No folders configured"
        })));
    }

    let event_sender = state.event_sender.clone();
    let db = state.db.clone();
    let folders_clone = folders_to_process.clone();

    std::thread::spawn(move || {
        if let Err(e) = db.clear_all_photos() {
            eprintln!("Failed to clear database: {}", e);
            let _ = event_sender.blocking_send(ProcessingEvent {
                event_type: "processing_error".to_string(),
                data: ProcessingData {
                    message: Some(format!("Failed to clear database: {}", e)),
                    phase: Some("error".to_string()),
                    ..Default::default()
                },
            });
            return;
        }

        let mut total_stats = (0usize, 0usize, 0usize, 0usize);

        for photos_dir in &folders_clone {
            if !photos_dir.exists() {
                eprintln!("⚠️ Folder not found: {}", photos_dir.display());
                let _ = event_sender.blocking_send(ProcessingEvent {
                    event_type: "processing_error".to_string(),
                    data: ProcessingData {
                        message: Some(format!("Folder not found: {}", photos_dir.display())),
                        phase: Some("error".to_string()),
                        ..Default::default()
                    },
                });
                continue;
            }

            match process_photos_with_stats(&db, photos_dir, false, false) {
                Ok((total_files, processed_count, no_gps_count, heic_count)) => {
                    total_stats.0 += total_files;
                    total_stats.1 += processed_count;
                    total_stats.2 += no_gps_count;
                    total_stats.3 += heic_count;
                }
                Err(e) => {
                    eprintln!("Processing error for {}: {}", photos_dir.display(), e);
                    let _ = event_sender.blocking_send(ProcessingEvent {
                        event_type: "processing_error".to_string(),
                        data: ProcessingData {
                            message: Some(format!(
                                "Processing failed for {}: {}",
                                photos_dir.display(),
                                e
                            )),
                            phase: Some("error".to_string()),
                            ..Default::default()
                        },
                    });
                }
            }
        }

        let _ = event_sender.blocking_send(ProcessingEvent {
            event_type: "processing_complete".to_string(),
            data: ProcessingData {
                total_files: Some(total_stats.0),
                processed: Some(total_stats.1),
                gps_found: Some(total_stats.1),
                no_gps: Some(total_stats.2),
                heic_files: Some(total_stats.3),
                skipped: Some(total_stats.0 - total_stats.1),
                message: Some(format!(
                    "Processing finished! Processed {} photos from {} folder(s)",
                    total_stats.1,
                    folders_clone.len()
                )),
                phase: Some("completed".to_string()),
                ..Default::default()
            },
        });
    });

    Ok(Json(serde_json::json!({
        "status": "started",
        "message": format!("Processing {} folder(s)", folders_to_process.len()),
        "count": folders_to_process.len()
    })))
}

pub async fn initiate_processing(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let event_sender = state.event_sender.clone();
    let db = state.db.clone();

    let folders_to_process = {
        let settings = state.settings.lock().await;
        settings
            .folders
            .iter()
            .filter_map(|f| f.as_ref().map(|s| std::path::Path::new(s).to_path_buf()))
            .collect::<Vec<_>>()
    };

    if folders_to_process.is_empty() {
        return Ok(Json(serde_json::json!({
            "status": "error",
            "message": "No folders configured"
        })));
    }

    let folders_clone = folders_to_process.clone();

    std::thread::spawn(move || {
        let mut total_stats = (0usize, 0usize, 0usize, 0usize);

        for photos_dir in &folders_clone {
            if !photos_dir.exists() {
                eprintln!("⚠️ Folder not found: {}", photos_dir.display());
                continue;
            }

            match process_photos_from_directory(&db, photos_dir) {
                Ok((total_files, processed_count, no_gps_count, heic_count)) => {
                    total_stats.0 += total_files;
                    total_stats.1 += processed_count;
                    total_stats.2 += no_gps_count;
                    total_stats.3 += heic_count;
                }
                Err(e) => {
                    eprintln!("Processing error for {}: {}", photos_dir.display(), e);
                }
            }
        }

        let _ = event_sender.blocking_send(ProcessingEvent {
            event_type: "processing_complete".to_string(),
            data: ProcessingData {
                total_files: Some(total_stats.0),
                processed: Some(total_stats.1),
                gps_found: Some(total_stats.1),
                no_gps: Some(total_stats.2),
                heic_files: Some(total_stats.3),
                skipped: Some(total_stats.0 - total_stats.1),
                message: Some(format!(
                    "Processing finished! Processed {} photos from {} folder(s)",
                    total_stats.1,
                    folders_clone.len()
                )),
                phase: Some("completed".to_string()),
                ..Default::default()
            },
        });
    });

    Ok(Json(serde_json::json!({
        "status": "started",
        "message": format!("Processing {} folder(s)", folders_to_process.len()),
        "count": folders_to_process.len()
    })))
}

pub async fn processing_events_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let (tx, rx) = mpsc::channel(100);

    let mut event_receiver = state.event_broadcast.subscribe();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                event = event_receiver.recv() => {
                    match event {
                        Ok(processing_event) => {
                            let sse_event = axum::response::sse::Event::default()
                                .json_data(&processing_event)
                                .unwrap_or_else(|_| axum::response::sse::Event::default().data("Error serializing event"));
                            if tx.send(Ok(sse_event)).await.is_err() { break; }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    });

    let stream = ReceiverStream { rx };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive-message"),
    )
}

pub async fn index_html() -> Html<&'static [u8]> {
    Html(INDEX_HTML)
}

pub async fn style_css() -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "text/css")
        .body(STYLE_CSS.to_vec().into())
        .expect("Failed to build CSS response")
}

pub async fn script_js() -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/javascript")
        .body(SCRIPT_JS.to_vec().into())
        .expect("Failed to build JS response")
}

pub async fn shutdown_app(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _ = state.shutdown_sender.send(());
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Server shutting down"
    })))
}

pub async fn select_folder_dialog(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let folder_paths = tokio::task::spawn_blocking(crate::utils::select_folders_native)
        .await
        .map_err(|e| {
            eprintln!("Task join error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !folder_paths.is_empty() {
        Ok(Json(serde_json::json!({
            "status": "success",
            "folder_paths": folder_paths,
            "count": folder_paths.len(),
            "message": if folder_paths.len() > 1 {
                format!("{} folders selected", folder_paths.len())
            } else {
                "Folder selected".to_string()
            }
        })))
    } else {
        Ok(Json(serde_json::json!({
            "status": "cancelled",
            "message": "Folder selection cancelled"
        })))
    }
}

pub async fn reveal_file(
    Json(file_path): Json<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use std::process::Command;
    let result = {
        #[cfg(target_os = "windows")]
        {
            let clean_path = file_path.replace("/", "\\");
            Command::new("explorer")
                .arg(format!("/select,{}", clean_path))
                .spawn()
        }
        #[cfg(target_os = "macos")]
        {
            Command::new("open").arg("-R").arg(&file_path).spawn()
        }
        #[cfg(target_os = "linux")]
        {
            Command::new("nautilus")
                .arg("--select")
                .arg(&file_path)
                .spawn()
                .or_else(|_| {
                    use std::path::Path;
                    let parent = Path::new(&file_path)
                        .parent()
                        .and_then(|p| p.to_str())
                        .unwrap_or(&file_path);
                    Command::new("xdg-open").arg(parent).spawn()
                })
        }
    };
    match result {
        Ok(_) => Ok(Json(serde_json::json!({"status": "success"}))),
        Err(e) => {
            eprintln!("❌ Failed to open file manager: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
