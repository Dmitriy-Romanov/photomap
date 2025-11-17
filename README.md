# PhotoMap Processor v0.6.1

A modern, high-performance photo mapping application with SQLite database storage and on-demand marker generation. Built with Rust for speed and reliability.

## ✨ Features

- **SQLite Database Storage**: Scalable storage for thousands of photos.
- **On-demand Generation**: No static thumbnail files are created; markers are generated as needed.
- **Native HEIC Support**: Full HEIC/HEIF support using a native Rust solution (no ImageMagick required).
- **Embedded Frontend**: The entire web interface is embedded in the binary for a true single-file executable.
- **Interactive Clustering**: Smart photo clustering with numbered markers.
- **Subfolder Support**: Handles duplicate filenames from different cameras.
- **700px Popups**: Large, detailed photo previews with metadata.
- **Real-time Processing**: Parallel processing for fast performance.
- **Simplified UX**: One-click folder selection and processing.
- **Cross-platform**: Works on Windows, macOS, and Linux.
- **Auto-restart**: Automatically processes the last selected folder on startup.

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
│   ├── server.rs        # HTTP API endpoints
│   ├── image_processing.rs # Image processing & thumbnail generation
│   ├── exif_parser.rs   # EXIF data extraction
│   ├── settings.rs      # Configuration management
│   └── ...
├── frontend/            # Embedded web interface files
│   ├── index.html
│   ├── style.css
│   └── script.js
├── photos/              # Your photo collection (git-ignored)
└── README.md
```

## 📈 Version History

### v0.6.1
- **Refactored Thumbnail Generation**: Now creates high-quality 120x120px JPG thumbnails with white padding for HiDPI/Retina displays.
- **Fixed HEIC Orientation**: Corrected a bug that caused some HEIC images to be rotated incorrectly.
- **Embedded Frontend**: The HTML, CSS, and JS are now embedded into the final binary using `rust-embed`.
- **Cleaned Up UI**: Removed unnecessary console output on startup.
- **Cleaned Up Documentation**: Removed several outdated markdown files.

### v0.6.0
- **Native HEIC Processing**: Replaced ImageMagick dependency with a pure Rust solution for HEIC processing.

*(Older version history has been condensed for brevity. See git history for full details.)*

## 📄 License

MIT License
