# PhotoMap Processor v3.0

A modern, high-performance photo mapping application with SQLite database storage and on-demand marker generation. Built with Rust for speed and reliability.

## ✨ Features

- **SQLite Database Storage** - Scalable storage for thousands of photos
- **On-demand Generation** - No static thumbnail files, markers generated as needed
- **HEIC Support** - Full HEIC/HEIF support with ImageMagick integration
- **Interactive Clustering** - Smart photo clustering with numbered markers
- **Subfolder Support** - Handle duplicate filenames from different cameras
- **700px Popups** - Large, detailed photo previews with metadata
- **Real-time Processing** - Parallel processing for fast performance
- **Cross-platform** - Windows, macOS, Linux support

## 🏗️ Architecture

### Modern Design (v3.0)

```
Photos (any subfolders)
  → Parallel EXIF extraction
  → SQLite database storage
  → HTTP server with API endpoints
  → Interactive web map with clustering
```

### Key Components

- **Database**: SQLite with optimized indexes for performance
- **Server**: Axum HTTP framework with async/await
- **Frontend**: Leaflet.js with MarkerCluster plugin
- **Processing**: Rayon parallel processing
- **Images**: Native parsing + ImageMagick HEIC conversion

## 📸 Supported Formats

- **Images**: JPG, JPEG, PNG, WebP, TIFF, BMP, GIF, HEIC, HEIF, AVIF
- **GPS Data**: EXIF GPS coordinate extraction
- **Output**: Interactive web map with real-time markers

## 🚀 Quick Start

### Prerequisites

- **Rust 1.70+**
- **ImageMagick** (for HEIC support, optional):
  ```bash
  # macOS
  brew install imagemagick

  # Ubuntu/Debian
  sudo apt-get install imagemagick

  # Windows
  # Download from https://imagemagick.org/script/download.php
  ```

### Installation

```bash
git clone https://github.com/your-repo/photomap.git
cd photomap
cargo build --release
```

### Usage

1. **Place photos** in the `photos/` directory (any subfolder structure)
2. **Run the application**:
   ```bash
   ./target/release/photomap_processor
   ```
3. **Open the map** at http://127.0.0.1:3001

## 📁 Project Structure

```
photomap/
├── src/
│   ├── main.rs              # Application entry point
│   ├── database.rs          # SQLite database operations
│   ├── server.rs            # HTTP API endpoints
│   ├── image_processing.rs  # Image processing & HEIC conversion
│   ├── exif_parser.rs       # EXIF data extraction
│   └── html_template.rs     # Web interface template
├── photos/                  # Your photo collection (git-ignored)
├── target/                  # Build output (git-ignored)
├── photomap.db             # SQLite database (git-ignored)
└── README.md
```

## 🔧 Configuration

### HEIC Support

The application automatically detects ImageMagick:
- ✅ **With ImageMagick**: Full HEIC processing with thumbnails
- ⚠️ **Without ImageMagick**: HEIC files are skipped

### Photo Organization

- **Subfolders supported**: Photos can be organized in any folder structure
- **Duplicate filenames**: Files with same names from different folders are handled uniquely
- **Relative paths**: Database uses relative paths for portability

## 🌐 API Endpoints

- `GET /` - Interactive map interface
- `GET /api/photos` - List all photos with GPS data
- `GET /api/marker/*path` - Generate 50x50px marker icon
- `GET /api/thumbnail/*path` - Generate 100x100px thumbnail
- `GET /convert-heic?filename=path` - Convert HEIC to JPEG

## 📊 Performance

- **Processing**: ~1.5ms per photo with parallel processing
- **Storage**: Efficient SQLite with indexes
- **Memory**: On-demand generation prevents memory issues
- **Scalability**: Tested with 10,000+ photos

## 🗺️ Map Features

### Interactive Clustering
- **Numbered clusters** show photo counts
- **Progressive detail** - zoom in to see individual photos
- **Smart grouping** based on proximity

### Photo Information
- **Large popups** (700px) for detailed viewing
- **Shooting date/time** from EXIF data
- **File information** and relative paths
- **Sector photo counts** - shows photos in current view

## 🛠️ Development

### Building

```bash
cargo build --release    # Optimized build
cargo run --release      # Run with release optimizations
```

### Code Organization

The codebase is organized into logical modules:
- **Database**: All SQLite operations and schema
- **Server**: HTTP handlers and API logic
- **Image Processing**: Thumbnail generation and HEIC conversion
- **EXIF Parsing**: GPS and metadata extraction
- **HTML Template**: Web interface generation

## 📈 Version History

### v3.0 (Current)
- ✅ SQLite database storage
- ✅ On-demand marker generation
- ✅ Subfolder and duplicate filename support
- ✅ 700px popups with metadata
- ✅ Modular codebase architecture

### v2.0
- ✅ Native HEIC/JPEG parsers
- ✅ Static HTML/JSON output
- ✅ Basic clustering support

### v1.0
- ✅ Basic GPS extraction
- ✅ Simple map generation

## 🤝 Contributing

Pull requests are welcome! Please ensure:

- Code follows Rust conventions
- All comments and documentation are in English
- Tests pass for new features
- Update documentation for API changes

## 📄 License

MIT License - see LICENSE file for details

## 🆘 Troubleshooting

### HEIC Files Not Processing
```bash
# Check if ImageMagick is installed
magick --version
# or
convert --version
```

### Performance Issues
- Ensure sufficient RAM for large photo collections
- Consider SSD storage for faster database operations
- Use release builds: `cargo build --release`

### Map Not Loading
- Check that the server is running on port 3001
- Verify photos have GPS data in EXIF
- Check browser console for JavaScript errors