# PhotoMap Processor v0.6.6

A modern, high-performance photo mapping application with SQLite database storage and on-demand marker generation. Built with Rust for speed and reliability.

## ✨ Key Technical Improvements (v0.6.6)

- **HiDPI Support**: Popup images are now generated at 2x resolution (1400px) for sharp display on Retina screens.
- **Performance**: 64x faster thumbnail generation using TurboJPEG scaling and Triangle filter.
- **Stability**: Robust error handling for invalid folder configurations (auto-clearing database).
- **Cross-Platform**: Full Windows compatibility with `sysinfo` for process management.

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
5.  **Select a folder** with photos to start processing.

## 📁 Project Structure

```
photomap/
├── src/                 # Rust source code
│   ├── main.rs          # Application entry point
│   ├── database.rs      # SQLite database operations
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
├── frontend/            # Embedded web interface files
├── log/                 # Log files (git-ignored)
├── photos/              # Your photo collection (git-ignored)
└── README.md
```

## 📈 Version History

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
