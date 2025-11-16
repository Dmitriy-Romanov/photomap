# PhotoMap Processor v0.4.0

A modern, high-performance photo mapping application with SQLite database storage and on-demand marker generation. Built with Rust for speed and reliability.

## ✨ Features

- **SQLite Database Storage** - Scalable storage for thousands of photos
- **On-demand Generation** - No static thumbnail files, markers generated as needed
- **HEIC Support** - Full HEIC/HEIF support with ImageMagick integration
- **Interactive Clustering** - Smart photo clustering with numbered markers
- **Subfolder Support** - Handle duplicate filenames from different cameras
- **700px Popups** - Large, detailed photo previews with metadata
- **Real-time Processing** - Parallel processing for fast performance
- **Simplified UX** - One-click folder selection and processing
- **Cross-platform** - Windows, macOS, Linux support
- **Folder Dialog** - Native folder selection with Cancel support
- **Auto-restart** - Automatically processes last selected folder on startup

## 🏗️ Architecture

### Modern Design (v0.4.0)

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
- **UX**: Unified folder selection with automatic processing

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

1. **Run the application**:
   ```bash
   ./target/release/photomap_processor
   ```
2. **Open the map** at http://127.0.0.1:3001
3. **Click "Обзор"** to select a folder with photos
4. **Processing starts automatically** after folder selection

## 📁 Project Structure

```
photomap/
├── src/
│   ├── main.rs              # Application entry point
│   ├── database.rs          # SQLite database operations
│   ├── server.rs            # HTTP API endpoints
│   ├── image_processing.rs  # Image processing & HEIC conversion
│   ├── exif_parser.rs       # EXIF data extraction
│   ├── html_template.rs     # Web interface template
│   ├── folder_picker.rs     # Cross-platform folder selection
│   └── settings.rs          # Configuration management
├── folder_dialog_helper/    # External helper for macOS folder dialogs
├── photos/                  # Your photo collection (git-ignored)
├── target/                  # Build output (git-ignored)
├── photomap.db             # SQLite database (git-ignored)
├── photomap.ini            # Configuration file (git-ignored)
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

### Configuration File (photomap.ini)

```
last_folder = "/path/to/last/selected/folder"
port = 3001
auto_open_browser = false
info_panel_width = 255
show_progress = true
```

## 🌐 API Endpoints

- `GET /` - Interactive map interface
- `GET /api/photos` - List all photos with GPS data
- `GET /api/marker/*path` - Generate 40x40px marker icon
- `GET /api/thumbnail/*path` - Generate 60x60px thumbnail
- `GET /convert-heic?filename=path` - Convert HEIC to JPEG
- `GET /api/settings` - Load current settings
- `POST /api/settings` - Save settings
- `POST /api/select-folder` - Select folder with automatic processing
- `POST /api/process` - Start photo processing
- `GET /api/events` - Real-time processing updates

## 📊 Performance

- **Processing**: ~1.5ms per photo with parallel processing
- **Storage**: Efficient SQLite with indexes
- **Memory**: On-demand generation prevents memory issues
- **Scalability**: Tested with 10,000+ photos
- **Binary Size**: ~3-4MB (depends on platform)

## 🗺️ Map Features

### Interactive Clustering
- **Numbered clusters** show photo counts
- **Progressive detail** - zoom in to see individual photos
- **Smart grouping** based on proximity

### Photo Information
- **Large popups** (700px) for detailed viewing
- **Shooting date/time** from EXIF data
- **File information** and relative paths
- **Visible photo counts** - shows photos in current map view

### User Interface
- **Simplified workflow**: One-click folder selection and processing
- **Real-time feedback**: Progress indicators and notifications
- **Automatic updates**: Map refreshes after processing completion
- **Statistics panel**: Shows total and visible photo counts

## 🖥️ User Experience

### New Simplified Workflow (v3.0)

1. **Launch application** - automatically loads last selected folder
2. **Click "Обзор"** - opens native folder selection dialog
3. **Select folder** - processing starts automatically
4. **View results** - interactive map with clustered photos

### Folder Selection Features

- **Native dialogs**: Uses OS-specific folder selection
- **Cancel support**: Properly handles user cancellation
- **Visual feedback**: Button changes to show current operation
- **Error handling**: Clear notifications for any issues

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
- **Folder Picker**: Cross-platform folder selection
- **Settings**: Configuration management

### Distribution

The application is distributed as:
- **Main binary**: `photomap_processor` (~3-4MB)
- **Helper binary**: `folder_dialog_helper` (~2MB, for macOS folder dialogs)

## 📈 Version History

### v3.0 (Current)
- ✅ SQLite database storage
- ✅ On-demand marker generation
- ✅ Subfolder and duplicate filename support
- ✅ 700px popups with metadata
- ✅ Modular codebase architecture
- ✅ Simplified UX with unified folder selection
- ✅ Cancel button support in folder dialogs
- ✅ Automatic processing on folder selection
- ✅ Cross-platform folder selection support

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

### Folder Dialog Issues
- **macOS**: The helper binary should be in `folder_dialog_helper/target/release/`
- **Windows/Linux**: Uses native system dialogs
- **Cancel button**: Properly handled in v3.0+

### Performance Issues
- Ensure sufficient RAM for large photo collections
- Consider SSD storage for faster database operations
- Use release builds: `cargo build --release`

### Map Not Loading
- Check that the server is running on port 3001
- Verify photos have GPS data in EXIF
- Check browser console for JavaScript errors

### Binary Size
- Current size is ~3-4MB for main binary
- Helper binary adds ~2MB for macOS folder dialogs
- Optimized for distribution and portability