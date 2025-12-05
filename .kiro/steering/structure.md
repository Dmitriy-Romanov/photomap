# Project Structure

## Directory Layout

```
photomap/
├── src/                      # Rust source code
│   ├── main.rs              # Application entry point & initialization
│   ├── constants.rs         # Application constants (e.g., POPUP_SIZE)
│   ├── database.rs          # In-memory database with bincode persistence
│   ├── processing.rs        # Photo scanning and parallel processing
│   ├── image_processing.rs  # Image resizing and format conversion
│   ├── geocoding.rs         # Reverse geocoding with KD-Tree
│   ├── settings.rs          # INI-based configuration management
│   ├── utils.rs             # Utility functions
│   ├── process_manager.rs   # Single instance enforcement
│   ├── exif_parser/         # EXIF metadata extraction
│   │   ├── mod.rs           # Module exports
│   │   ├── heic.rs          # HEIC/HEIF parser
│   │   ├── jpeg.rs          # JPEG parser with custom GPS extraction
│   │   └── generic.rs       # Shared EXIF utilities
│   └── server/              # HTTP server (Axum)
│       ├── mod.rs           # Router setup and server initialization
│       ├── handlers.rs      # API endpoint handlers
│       ├── state.rs         # Shared application state
│       └── events.rs        # SSE event definitions
├── frontend/                # Web interface (embedded via rust-embed)
│   ├── index.html           # Main HTML structure
│   ├── script.js            # Leaflet map and UI logic
│   └── style.css            # Styling and layout
├── exif_parser_test/        # Standalone EXIF debugging tool
├── photos/                  # User photo directories (gitignored)
├── log/                     # Application logs (gitignored)
├── target/                  # Cargo build artifacts (gitignored)
├── Cargo.toml               # Rust dependencies and build config
├── README.md                # User documentation
└── PROJECT_MAP.md           # Technical architecture documentation
```

## Module Organization

### Core Application (`src/`)

**main.rs**
- Application bootstrap and initialization
- Logging setup with custom timestamp format
- Database initialization and cache loading
- Multi-folder processing on startup
- HTTP server launch
- Browser auto-start (optional)

**database.rs**
- `Database` struct with `Arc<RwLock<Vec<PhotoMetadata>>>`
- In-memory photo storage
- Batch insert operations for performance
- Binary cache persistence with version checking
- Smart cache invalidation on folder path changes

**processing.rs**
- `process_photos_with_stats()` - Main processing entry point
- Parallel file scanning using `rayon::par_bridge()`
- EXIF extraction and GPS validation
- Batch database insertion
- Processing statistics and timing

**image_processing.rs**
- `create_scaled_image_in_memory()` - Dynamic image resizing
- `convert_heic_to_jpeg()` - Format conversion
- Uses `turbojpeg` for JPEG operations (20-30% faster)
- Triangle filter for quality/speed balance

**geocoding.rs**
- `ReverseGeocoder` with embedded GeoNames database
- KD-Tree spatial indexing for O(log n) lookups
- Lazy initialization to avoid blocking startup
- Returns city and country codes

### EXIF Parser (`src/exif_parser/`)

**heic.rs**
- HEIC/HEIF metadata extraction via `libheif-rs`
- GPS coordinate parsing
- DateTime extraction

**jpeg.rs**
- Custom JPEG EXIF parser
- Handles malformed EXIF data (Lightroom compatibility)
- 24% faster than generic parser

**generic.rs**
- `get_gps_coord()` - GPS coordinate extraction with hemisphere handling
- `get_datetime_from_exif()` - DateTime parsing with fallbacks
- `apply_exif_orientation()` - Image rotation correction

### HTTP Server (`src/server/`)

**mod.rs**
- Axum router configuration
- Static file serving for frontend
- API route definitions
- Graceful shutdown handling

**handlers.rs**
- `get_all_photos` - Returns photo metadata with location names
- `serve_processed_image` - On-demand thumbnail/marker generation
- `convert_heic` - HEIC to JPEG conversion endpoint
- `get_settings` / `update_settings` - Configuration management
- `reprocess_photos` - Trigger photo reprocessing
- `processing_events_stream` - SSE for real-time progress
- `shutdown` - Graceful application shutdown

**state.rs**
- `AppState` struct shared across handlers
- Contains database, settings, event channels

**events.rs**
- `ProcessingEvent` enum for SSE messages
- `ProcessingData` struct for progress updates

### Frontend (`frontend/`)

**index.html**
- Leaflet map container
- Draggable control panel
- Year range filters
- Statistics display
- Modal gallery for clusters

**script.js**
- Map initialization with clustering
- API communication (centralized endpoints)
- Real-time SSE event handling
- Marker filtering and popup generation
- Heatmap and route visualization
- JSDoc documentation throughout

**style.css**
- Glassmorphism panel design
- Responsive layout
- SVG sprite system
- Utility classes for common styles

## Code Conventions

### Rust Style
- Use `anyhow::Result` for error handling
- Prefer `Arc<RwLock<>>` for shared mutable state
- Use `tracing` macros for logging (info!, warn!, error!)
- Async functions use `tokio::spawn` for background tasks
- Batch operations for database performance

### Error Handling
- Return `Result<T>` from fallible functions
- Use `.context()` to add error context
- Log errors with appropriate severity
- Graceful degradation (e.g., skip photos without GPS)

### Logging Format
- Custom timestamp: `DD HH:MM:SS`
- Emoji prefixes for visual scanning (🚀, ✅, ⚠️, ❌)
- Structured logging with `tracing` crate
- Default log level: INFO (filters dependency noise)

### Performance Patterns
- Parallel processing with `rayon` for CPU-bound tasks
- Batch database operations to reduce lock contention
- Lazy initialization for expensive resources (geocoder)
- Binary cache to avoid reprocessing on startup
- On-demand image generation (not pre-generated)

## File Naming

- Rust modules: `snake_case.rs`
- Frontend files: `lowercase.html/js/css`
- Cache files: `photos_v1.bin` (versioned)
- Config files: `settings.ini`
- Log files: `photomap.log`

## Critical Design Patterns

### Thread Safety
- Database uses `Arc<RwLock<Vec<PhotoMetadata>>>` for concurrent access
- Settings use `Arc<Mutex<Settings>>` for configuration updates
- Read-heavy workloads prefer `RwLock` over `Mutex`

### Cache Strategy
- Cache version number prevents loading incompatible formats
- Cache is invalidated if folder paths don't match 100%
- Old cache files (`photos.db`, `photos.bin`) are auto-deleted on startup

### Processing Flow
1. Scan directories with `ignore` crate (respects .gitignore)
2. Filter by supported extensions (jpg, jpeg, heic, heif, avif)
3. Parallel EXIF extraction with `rayon`
4. Batch insert to database (reduces lock contention)
5. Save cache with folder paths for validation

### Image Serving
- Images are NOT pre-generated or stored
- Generated on-demand when requested via API
- Cached by browser (HTTP caching headers)
- HEIC files converted to JPEG on-the-fly

### Error Recovery
- Photos without GPS are logged and skipped (not errors)
- Malformed EXIF triggers fallback parsers
- Database corruption triggers auto-clear and reprocess
- Single instance enforcement prevents port conflicts
