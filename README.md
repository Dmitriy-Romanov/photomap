# PhotoMap Processor v0.6.4

A modern, high-performance photo mapping application with SQLite database storage and on-demand marker generation. Built with Rust for speed and reliability.

## ✨ Key Technical Improvements (v0.6.2 -> v0.6.3)

This version includes a major internal refactoring to improve robustness, scalability, and maintainability.

- **Robust Metadata Parsers**: Replaced fragile, custom-built EXIF parsers with professional, standard libraries (`kamadak-exif` and `libheif-rs`), significantly increasing reliability.
- **Scalable File Processing**: The file processing engine was rewritten to use a memory-efficient iterator-based approach. The application can now smoothly process virtually unlimited numbers of photos without high memory usage.
- **Professional Logging**: Implemented the `tracing` framework for structured, configurable logging. All output is now sent to both the console and a daily rotating log file located in the `log/` directory.

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

### v0.6.3
- **Major Refactoring**: Implemented robust EXIF parsers, a scalable file processing engine, and a professional logging framework (`tracing`). See "Key Technical Improvements".
- **Bug Fixes**: Resolved compilation issues and a regression where certain HEIC files were not processed correctly.
- **Documentation**: Updated `PROJECT_MAP.md` and `README.md` to reflect the current architecture.

### v0.6.2
- **Enhanced UI Edition**: No specific code changes, version bump for context.

### v0.6.1
- **Refactored Thumbnail Generation**: Now creates high-quality 120x120px JPG thumbnails with white padding for HiDPI/Retina displays.
- **Fixed HEIC Orientation**: Corrected a bug that caused some HEIC images to be rotated incorrectly.
- **Embedded Frontend**: The HTML, CSS, and JS are now embedded into the final binary using `rust-embed`.
- **Cleaned Up UI**: Removed unnecessary console output on startup.
- **Cleaned Up Documentation**: Removed several outdated markdown files.

*(Older version history has been condensed for brevity. See git history for full details.)*

## 📄 License

MIT License
