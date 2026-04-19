# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PhotoMap Processor is a high-performance Rust application for processing photos, extracting EXIF/GPS metadata, and displaying them on an interactive map. Uses an in-memory database with binary disk cache for instant loading of large collections.

## Common Development Commands

```bash
# Build release version (optimized)
cargo build --release

# Run application
./target/release/photomap_processor

# Run development version
cargo run

# Run tests
cargo test

# Linting
cargo clippy

# Code formatting
cargo fmt
```

After startup, web interface is available at http://127.0.0.1:3001

## Architecture

### Backend (Rust)

- **main.rs** — entry point. Initializes database, settings, starts HTTP server. Handles cache loading on startup.
- **server/** — Axum HTTP server with API for frontend
  - `mod.rs` — router and server startup on port 3001
  - `handlers.rs` — API handlers (photos, images, settings, processing, shutdown)
  - `state.rs` — AppState with Arc<RwLock<>> for sharing
  - `events.rs` — SSE events for real-time updates
- **database.rs** — in-memory database (Vec<PhotoMetadata>) with persistence via bincode in `photos.bin`
- **processing.rs** — folder scanning and photo processing coordination
- **exif_parser/** — metadata extraction module
  - `jpeg.rs` — EXIF from JPEG via kamadak-exif
  - `heic.rs` — EXIF from HEIC via libheif-rs
  - `gps_parser.rs` — low-level GPS parser for corrupted EXIF
  - `generic.rs` — common functions for GPS coordinates and dates
- **image_processing.rs** — thumbnail creation, HEIC→JPEG conversion, uses turbojpeg for speed
- **geocoding.rs** — offline reverse geocoding via embedded GeoNames database (140k+ cities) with KD-Tree index
- **settings.rs** — settings management (INI file), stores up to 5 folders
- **folder_picker.rs** — native folder selection dialogs (macOS/Windows/Linux)

### Frontend (embedded)

Frontend is located in `frontend/` and embedded into binary via `std::include_bytes!`.

- **index.html** — page structure with Leaflet map
- **script.js** — map logic, markers, clustering, gallery. Divided into sections: API, DataService, MapController, UIController
- **style.css** — styles for map, popups, draggable panel

### Data Flow

1. On startup, loads cache `photos.bin` if folder paths match
2. If cache invalid — scans folders, extracts EXIF, metadata saved to in-memory DB
3. Frontend requests `/api/photos` — receives JSON with metadata
4. Images generated on-demand when requesting `/api/marker/*`, `/api/thumbnail/*`, `/api/popup/*`
5. SSE `/api/events` used for processing progress

## Key Technical Details

- **Multi-folder support**: up to 5 folders simultaneously, stored in settings as array
- **Lazy geocoding**: geocoding module initializes in background on startup
- **Image sizes** (constants.rs): MARKER=40px, THUMBNAIL=120px, GALLERY=240px, POPUP=1400px
- **Cross-platform**: Windows/macOS/Linux, uses different native dialogs for each platform
- **Single instance**: process_manager kills existing processes before starting
- **Graceful shutdown**: `/api/shutdown` stops server gracefully

## Configuration

Settings file automatically created in:
- macOS: `~/Library/Application Support/PhotoMap/config.ini`
- Windows: `%APPDATA%\PhotoMap\config.ini`
- Linux: `~/.config/PhotoMap/config.ini`

Contains: folders, panel position, toggles (coordinates, routes, heatmap, browser autostart)
