# Technology Stack

## Build System

- **Language**: Rust (edition 2021, version 1.70+)
- **Build Tool**: Cargo
- **Package Manager**: Cargo

## Core Dependencies

### Web Server & Async Runtime
- `tokio` (1.0) - Async runtime with multi-threaded executor
- `axum` (0.7) - Web framework for HTTP server
- `tower` & `tower-http` (0.5) - Middleware (CORS, compression, static files)

### Image Processing
- `image` (0.25) - Image manipulation (JPEG, PNG support only)
- `turbojpeg` (1.3.3) - Fast JPEG encoding/decoding with hardware acceleration
- `libheif-rs` (2.0.0) - HEIC/HEIF format support
- `exif` (kamadak-exif 0.6) - EXIF metadata extraction

### Data & Serialization
- `serde` & `serde_json` (1.0) - Serialization framework
- `bincode` (1.3) - Binary serialization for database cache
- `rust-embed` (8.0.0) - Embed frontend assets into binary

### Performance & Concurrency
- `rayon` (1.8) - Data parallelism for photo processing
- `ignore` (0.4) - Fast directory traversal with gitignore support

### Geospatial
- `kdtree` (0.7) - Spatial indexing for reverse geocoding
- `flate2` (1.0) - Gzip compression for embedded geodata

### Utilities
- `anyhow` (1.0) - Error handling
- `chrono` (0.4) - Date/time handling
- `tracing` & `tracing-subscriber` (0.1, 0.3) - Structured logging

## Common Commands

### Development
```bash
# Build debug version
cargo build

# Build release version (optimized)
cargo build --release

# Run application
cargo run

# Run with release optimizations
cargo run --release
```

### Testing & Quality
```bash
# Run tests
cargo test

# Check code without building
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Deployment
```bash
# Build optimized binary
cargo build --release

# Binary location
./target/release/photomap_processor
```

### Running the Application
```bash
# After building, run the binary
./target/release/photomap_processor

# Application starts on http://127.0.0.1:3001
```

## Build Configuration

The project uses aggressive size optimizations in `Cargo.toml`:
- `opt-level = "z"` - Optimize for binary size
- `lto = "fat"` - Link-time optimization
- `codegen-units = 1` - Single codegen unit for better optimization
- `strip = true` - Strip debug symbols
- `panic = "abort"` - Smaller panic handler

Current binary size: ~3.5MB (includes embedded frontend and geodata)

## Platform Support

- **Primary**: macOS (native folder picker implemented)
- **Supported**: Linux, Windows (cross-platform compatible)
- **Architecture**: x86_64, ARM64

## Important Notes for Development

### Performance Considerations
- Photos without GPS data are skipped (not stored in database)
- HEIC files require `libheif-rs` which may need system dependencies
- The app uses single-instance enforcement (kills existing processes on startup)
- Cache invalidation happens automatically when folder paths change

### Common Issues
- **Windows**: Path handling uses forward slashes internally, converted from backslashes
- **HEIC Support**: Requires libheif system library on Linux/Windows
- **Large Collections**: 20k+ photos load instantly from cache, ~30-60s to process from scratch
- **Memory Usage**: In-memory database keeps all metadata in RAM (minimal footprint)

### API Endpoints
- `GET /api/photos` - All photo metadata with location names
- `GET /api/marker/:path` - Dynamically generated marker image
- `GET /api/thumbnail/:path` - Dynamically generated thumbnail
- `GET /api/convert-heic/:path` - HEIC to JPEG conversion
- `POST /api/reprocess` - Trigger photo reprocessing
- `GET /api/events` - SSE stream for processing progress
- `POST /api/shutdown` - Graceful shutdown
