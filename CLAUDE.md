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

# Run on a custom local port
cargo run -- --port 3002

# Run tests
cargo test

# Linting
cargo clippy

# Code formatting
cargo fmt
```

After startup, the web interface is available at http://127.0.0.1:3001 by default.

## Architecture

### Backend (Rust)

- **main.rs** — entry point. Parses `--port`, initializes database/settings/events, starts HTTP server. Handles cache loading on startup.
- **server/** — Axum HTTP server with API for frontend
  - `mod.rs` — router, localhost-only CORS, compression, and server startup on the configured port
  - `handlers.rs` — API handlers (photos, images, settings, processing, shutdown)
  - `state.rs` — AppState with database, `Arc<tokio::sync::Mutex<Settings>>`, mpsc processing events, broadcast SSE, and shutdown channel
  - `events.rs` — SSE events for real-time updates
- **database.rs** — in-memory database (`HashMap<String, PhotoMetadata>`) with persistence via bounded bincode in `photos_v1.bin`
- **processing.rs** — folder scanning and photo processing coordination
- **exif_parser/** — metadata extraction module
  - `jpeg.rs` — EXIF from JPEG via kamadak-exif
  - `heic.rs` — EXIF from HEIC via libheif-rs
  - `gps_parser.rs` — low-level GPS parser for corrupted EXIF
  - `generic.rs` — common functions for GPS coordinates and dates
- **image_processing.rs** — thumbnail creation, HEIC→JPEG conversion, uses turbojpeg for speed and guarded temp-file cleanup
- **geocoding.rs** — offline reverse geocoding via embedded GeoNames database (68k+ cities)
- **settings.rs** — settings management (INI file), stores up to 5 folders
- **utils.rs** — app data paths, browser launch, and native folder selection dialogs (macOS/Windows/Linux)

### Frontend (embedded)

Frontend is located in `frontend/` and embedded into binary via `std::include_bytes!`.

- **index.html** — page structure with Leaflet map
- **script.js** — map logic, markers, clustering, gallery. Divided into sections: API, DataService, MapController, UIController
- **style.css** — styles for map, popups, draggable panel

### Data Flow

1. On startup, loads cache `photos_v1.bin` if folder paths match
2. If cache invalid — scans folders, extracts EXIF, metadata saved to in-memory DB
3. Frontend requests `/api/photos` — receives JSON with metadata
4. Images generated on-demand when requesting `/api/marker/*`, `/api/thumbnail/*`, `/api/popup/*`
5. Processing events flow through an internal mpsc queue and are broadcast to SSE `/api/events`

## Key Technical Details

- **Multi-folder support**: up to 5 folders simultaneously, stored in settings as array
- **Lazy geocoding**: geocoding module initializes in background on startup
- **Dynamic port**: default 3001, override with `-p`/`--port <port>`
- **Indexed image lookup**: image routes use O(1) relative-path lookups
- **Image sizes** (constants.rs): MARKER=40px, THUMBNAIL=120px, GALLERY=240px, POPUP=1400px
- **Cross-platform**: Windows/macOS/Linux, uses different native dialogs for each platform
- **Single instance**: process_manager kills existing processes before starting
- **Graceful shutdown**: `/api/shutdown` stops server gracefully

## Configuration

Settings file automatically created in:
- macOS: `~/Library/Application Support/PhotoMap/photomap.ini`
- Windows: `%APPDATA%\PhotoMap\photomap.ini`
- Linux: `~/.local/share/PhotoMap/photomap.ini` by default, or `$XDG_DATA_HOME/PhotoMap/photomap.ini` when `XDG_DATA_HOME` is set

Contains: folders, panel position, toggles (coordinates, routes, heatmap, browser autostart)
