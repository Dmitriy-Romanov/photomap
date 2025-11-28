# PhotoMap Processor v0.9.5

A modern, high-performance photo mapping application with SQLite database storage and on-demand marker generation. Built with Rust for speed and reliability.

## ✨ Latest Improvements (v0.9.5)

- **Frontend Architecture**: Refactored `script.js` with centralized API endpoints and improved code organization (JSDoc).
- **Security & Reliability**: Safer path handling for Windows (removed inline `onclick` handlers), fixed path escaping issues.
- **UI Optimization**: Implemented SVG sprite system for cleaner HTML, refactored inline styles to CSS utility classes.
- **Visual Polish**: Adjusted panel width and column layout, fixed font weight consistency, fixed typos.
- **Code Quality**: Comprehensive JSDoc documentation for better maintainability.

## 🚀 Quick Start

### Prerequisites

- **Rust 1.70+**

### Installation & Usage

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/Dmitriy-Romanov/photomap.git
    cd photomap
    ```
2.  **Build the application**:
    ```bash
    cargo build --release
    ```
3.  **Run the application**:
    ```bash
    ./target/release/photomap_processor
    ```
4.  **Open the map** in your browser at [http://127.0.0.1:3001](http://127.0.0.1:3001).
5.  **Select folders** with photos to start processing (up to 5 folders).

## 📁 Project Structure

```
photomap/
├── src/                 # Rust source code
│   ├── main.rs          # Application entry point
│   ├── database.rs      # In-memory database operations
│   ├── processing.rs    # Core photo processing logic
│   ├── image_processing.rs # Image manipulation
│   ├── server/          # HTTP Server (Axum)
│   │   ├── mod.rs
│   │   ├── handlers.rs
│   │   └── ...
│   ├── exif_parser/     # EXIF metadata extraction
│   │   ├── mod.rs
│   │   ├── heic.rs
│   │   └── jpeg.rs
│   └── ...
├── exif_parser_test/    # Debugging tool for EXIF parsing
│   ├── src/
│   ├── README.md
│   └── ...
├── frontend/            # Embedded web interface files
├── log/                 # Log files (git-ignored)
├── photos/              # Your photo collection (git-ignored)
└── README.md
```

## 📈 Version History

### v0.9.5 - Frontend Architecture & Polish
- **Frontend Architecture**: Refactored `script.js` with centralized API endpoints and improved code organization (JSDoc).
- **Security & Reliability**: Safer path handling for Windows (removed inline `onclick` handlers), fixed path escaping issues.
- **UI Optimization**: Implemented SVG sprite system for cleaner HTML, refactored inline styles to CSS utility classes.
- **Visual Polish**: Adjusted panel width (540px) and column layout, fixed font weight consistency, fixed typos.
- **Code Quality**: Comprehensive JSDoc documentation for better maintainability.

### v0.9.3 - Multi-Folder & Smart Cache
- **Multi-Folder Support**: Select and process up to 5 photo folders simultaneously with native OS dialogs
- **Smart Cache v1**: Automatic cleanup of incompatible cache files, prevents crashes from format changes
- **Improved UX**: Better Windows folder selection prompts ("Add folder 2? (Cancel = Done)")
- **Code Cleanup**: Removed all legacy single-folder code, eliminated warnings
- **Bug Fixes**: Fixed database clearing, frontend settings loading, SSE events after reprocessing

### v0.9.1 - UI Polish & Smooth Zoom
- **Smooth Scroll Zoom**: Configured Leaflet with half-zoom steps (0.5) and reduced sensitivity for precise, comfortable navigation
- **Font Consistency**: Unified all panel label fonts to 15px for better cross-platform readability
- **Year Filter Fix**: Fixed year input validation - fields no longer interfere with each other, real-time validation
- **UI Alignment**: Adjusted label spacing and padding for better visual alignment
- **Double-Click Reset**: Added double-click on panel to reset to default position

### v0.9.0 - UI Redesign & Interaction
- **Redesigned Control Panel**: Complete rebuild of UI panel with modern row-based layout
- **Draggable Panel**: Drag the panel to reposition it - automatically saves position to config
- **User Location Marker**: Green marker shows your current location, "Where I" button centers map on you
- **Boundary Protection**: Panel automatically stays within viewport and resets if moved off-screen
- **Visual Improvements**: Added subtle gray backgrounds to panel rows for better visual separation
- **Compact Inputs**: Narrower year inputs (60px) optimized for 4-digit years
- **CSS Cleanup**: Removed duplicate definitions and unnecessary comments

### v0.8.2 - Visualization & Polish
- **Heatmap & Routes**: Added visual layers to see photo density and travel paths.
- **Performance**: Added JSON compression for efficient handling of large photo collections.
- **Browser Autostart**: Added toggle to control automatic browser launch.
- **UI Polish**: Improved typography (Sentence case) and added GitHub link.
- **Note**: Binary size increased to ~3.5MB due to new visualization features and compression support.
- **CI Fixes**: Fixed Windows and macOS build failures.

### v0.8.1 - Performance & Size Optimizations
- **JPEG Encoding**: Unified all image types to use `turbojpeg` for 20-30% faster popup generation
- **Process Management**: Replaced `sysinfo` library with native OS commands (pgrep/tasklist)
- **MIME Detection**: Replaced `mime_guess` with simple pattern matching function
- **Binary Size**: Reduced from 2.55MB to 2.3MB (-10%)
- **UI**: Modern redesign with system fonts and compact spacing

### v0.8.0 - Instant Startup & Persistence
- **Binary Cache**: Implemented `bincode` persistence (`photos.bin`). The application now saves the database state to disk and reloads it instantly on startup if the source folder is unchanged.
- **UI Fixes**: Fixed "Open" button resizing glitch by enforcing minimum width.
- **Optimization**: Zero-latency startup for large collections (20k+ photos).

### v0.7.4 - Cluster Gallery & UI Polish
- **Cluster Gallery**: New modal interface for viewing large clusters (10+ photos), replacing the "spiderfy" effect.
- **Pagination**: Added pagination (28 items/page) to the gallery for better performance with hundreds of photos.
- **UI Updates**: "Source Folder" input is now editable and full-width. Added "Process" button for quick re-runs.
- **Thumbnails**: Standardized on 240x240px square thumbnails with white padding for non-square images.

### v0.7.3 - In-Memory Database & Logging
- **In-Memory Database**: Complete migration from SQLite (`rusqlite`) to `Arc<RwLock<Vec<PhotoMetadata>>>`. Solved Windows file locking issues (`SQLITE_BUSY`).
- **Logging**: Standardized log format (`DD HH:MM:SS`) and removed noise from dependencies (`ignore` crate).
- **Cleanup**: Removed migration code and file-based DB logic.

### v0.7.2 - Performance & Parser Edition
- **Database Optimization**: Batch inserts + WAL mode for significantly faster photo processing.
- **Custom GPS Parser**: Low-level parser for malformed EXIF (24% faster, handles Lightroom files).
- **Smart EXIF Fallback**: Multi-tier parsing strategy with `continue_on_error` and custom byte reader.
- **Enhanced Test Tool**: Auto-copy failed files, performance benchmarking.

### v0.7.1 - Windows & Exif Stability
- **Windows Fixes**: Fixed folder dialog focus and Russian path encoding issues.
- **Exif Improvements**: Added fallback logic for misnamed HEIC files (Xiaomi bug).
- **New Tool**: Added `exif_parser_test` for verifying EXIF extraction accuracy.
- **Optimization**: Removed `open` crate dependency for smaller binary size.

### v0.6.7 - UI Redesign Edition
- **Modern Info Panel**: Redesigned with glassmorphism, translucent background (560px width), and optimized positioning.
- **Map Coordinates**: Real-time display of map center coordinates in header with toggle checkbox control.
- **Crosshair**: Optional center crosshair for precise navigation (toggleable with coordinates).
- **Improved Layout**: Statistics moved inline with year range, compact year inputs with overall min/max display.
- **Better Readability**: Increased font sizes by 2px across the panel for improved legibility.
- **Code Optimization**: Removed duplicate CSS definitions, unused styles, and cleaned up coordinate update logic.

### v0.6.6 - UI & UX Enhancement Edition
- **Year Range Filter**: Filter photos by year range with dynamic min/max labels.
- **Graceful Shutdown**: "Close map" button to safely shut down the application.
- **UI Improvements**: Fixed minimized window height, improved layout.
- **Optimization**: Reduced binary size by removing duplicate dependencies.

### v0.6.5 - Stability & Performance Edition
- **Thumbnail Optimization**: 64x faster decoding using TurboJPEG scaling + Triangle filter.
- **HiDPI Popups**: Increased popup resolution to 1400px for Retina displays.
- **Robust Error Handling**: Database auto-clearing on invalid config.
- **UI Safety**: Prevents selecting non-existent folders.

### v0.6.4 - Windows Compatibility Edition
- **Cross-Platform**: Replaced `pgrep`/`kill` with `sysinfo` for Windows support.
- **Cleanup**: Removed legacy Unix-specific code.

### v0.6.3
- **Major Refactoring**: Implemented robust EXIF parsers, a scalable file processing engine, and a professional logging framework (`tracing`). See "Key Technical Improvements".
- **Bug Fixes**: Resolved compilation issues and a regression where certain HEIC files were not processed correctly.
- **Documentation**: Updated `PROJECT_MAP.md` and `README.md` to reflect the current architecture.

## 📄 License

MIT License
