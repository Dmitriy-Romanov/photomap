use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{header, StatusCode},
    response::{Html, Json, Response},
    routing::get,
    Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, services::ServeDir};

use crate::database::{Database, ImageMetadata};
use crate::image_processing::{create_marker_icon_in_memory, create_thumbnail_in_memory, convert_heic_to_jpeg};
use crate::html_template::get_map_html;

// Application state for sharing database
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub has_heic_support: bool,
}

// HTTP API Handlers
pub async fn get_all_photos(State(state): State<AppState>) -> Result<Json<Vec<ImageMetadata>>, StatusCode> {
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
        }
    }).collect();

    Ok(Json(api_photos))
}

pub async fn get_marker_image(
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>
) -> Result<Response, StatusCode> {
    // Get photo file path from database
    let photos = state.db.get_all_photos()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let photo = photos.into_iter()
        .find(|p| p.relative_path == filename)
        .ok_or(StatusCode::NOT_FOUND)?;

    // For HEIC files, redirect to converted JPEG with proper size parameter
    if photo.is_heic {
        if state.has_heic_support {
            // Redirect to the converted HEIC image (served as JPEG)
            let redirect_url = format!("/convert-heic?filename={}&size=marker", filename);
            return Ok(Response::builder()
                .status(StatusCode::FOUND)
                .header(header::LOCATION, redirect_url)
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body("Redirecting to converted image".into())
                .unwrap());
        } else {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    // Generate marker icon on-demand for non-HEIC files
    let png_data = create_marker_icon_in_memory(std::path::Path::new(&photo.file_path))
        .map_err(|e| {
            eprintln!("Failed to create marker for {}: {}", filename, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(png_data.into())
        .unwrap())
}

pub async fn get_thumbnail_image(
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>
) -> Result<Response, StatusCode> {
    // Get photo file path from database
    let photos = state.db.get_all_photos()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let photo = photos.into_iter()
        .find(|p| p.relative_path == filename)
        .ok_or(StatusCode::NOT_FOUND)?;

    // For HEIC files, redirect to converted JPEG with proper size parameter
    if photo.is_heic {
        if state.has_heic_support {
            // Redirect to the converted HEIC image (served as JPEG)
            let redirect_url = format!("/convert-heic?filename={}&size=thumbnail", filename);
            return Ok(Response::builder()
                .status(StatusCode::FOUND)
                .header(header::LOCATION, redirect_url)
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body("Redirecting to converted image".into())
                .unwrap());
        } else {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    // Generate thumbnail (larger than marker) on-demand for non-HEIC files
    let png_data = create_thumbnail_in_memory(std::path::Path::new(&photo.file_path))
        .map_err(|e| {
            eprintln!("Failed to create thumbnail for {}: {}", filename, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(png_data.into())
        .unwrap())
}

pub async fn convert_heic(
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

pub async fn serve_map_html(State(state): State<AppState>) -> Html<String> {
    get_map_html(state.has_heic_support)
}

// Create the main application router
pub fn create_app(state: AppState) -> Router {
    // Используем абсолютный путь к папке с фотографиями
    let photos_path = std::env::current_dir()
        .unwrap_or_default()
        .join("photos");
    
    Router::new()
        .route("/", get(serve_map_html))
        .route("/api/photos", get(get_all_photos))
        .route("/api/marker/*filename", get(get_marker_image))
        .route("/api/thumbnail/*filename", get(get_thumbnail_image))
        .route("/convert-heic", get(convert_heic))
        .nest_service("/photos", ServeDir::new(photos_path))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
        )
        .with_state(state)
}

pub async fn start_server(state: AppState) -> Result<()> {
    let app = create_app(state);
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    let listener = TcpListener::bind(addr).await?;

    println!("   🌐 Server running at http://127.0.0.1:3001");
    println!("   📸 Enhanced map with clustering available at http://127.0.0.1:3001");
    println!("   🗺️  API endpoints:");
    println!("      - GET /api/photos - List all photos with GPS data");
    println!("      - GET /api/marker/<filename> - Generate 50x50px marker icon");
    println!("      - GET /api/thumbnail/<filename> - Generate 100x100px thumbnail");
    println!("      - GET /convert-heic?filename=<name> - Convert HEIC to JPEG");
    println!("   ✅ Enhanced: 700px popups + sector photo count + HEIC support!");

    axum::serve(listener, app).await?;
    Ok(())
}