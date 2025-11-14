# PhotoMap Processor

A utility for creating interactive photo maps from GPS EXIF data. Built with Rust and parallel processing.

## Features

- **Native HEIC/JPEG parsing** - No external dependencies required
- **GPS extraction** - Extracts GPS coordinates and shooting time from EXIF data
- **Parallel processing** - Fast processing of large photo collections
- **Interactive maps** - Creates beautiful Leaflet.js maps with photo clusters
- **Thumbnail generation** - Creates optimized thumbnails for all supported formats

## Supported Formats

- **Images**: JPG, JPEG, PNG, WebP, TIFF, BMP, GIF, **HEIC**, HEIF, AVIF
- **Output**: Interactive HTML map + JSON data

## Installation

### From Source

```bash
git clone https://github.com/Dmitriy-Romanov/photomap.git
cd photomap
cargo build --release
```

The compiled binary will be at `target/release/photomap_processor`.

## Usage

```bash
# Run from current directory
./target/release/photomap_processor

# Open the generated map
open map.html
```

### Output Files

- `map.html` - Interactive map with all photos
- `geodata.js` - GPS data in JSON format
- `.thumbnails/` - Generated thumbnails

## Architecture

### Native Parsers

The project uses custom parsers for major formats:

- **HEIC Parser**: Direct binary structure parsing
- **JPEG Parser**: APP1 segment EXIF extraction
- **Cross-platform**: Works on Windows, macOS, Linux

### Size Comparison

| Version | Size (Windows) | Dependencies |
|---------|----------------|-------------|
| With libheif | 9.4MB | External libraries required |
| **Native parsers** | **~3.8MB** | **No dependencies** |

## Technical Details

- **Language**: Rust
- **Parallelism**: Rayon for concurrent file processing
- **Mapping**: Leaflet.js with MarkerCluster
- **Thumbnails**: ImageMagick/fallback system
- **EXIF**: Custom parsing + standard library

## Development

### Build Requirements

- Rust 1.70+
- (Optional) ImageMagick for HEIC thumbnails

### Building

```bash
cargo build --release  # Standard build
```

## License

MIT License

## Contributing

Pull requests are welcome! Please ensure:

- Code follows Rust conventions
- Comments and documentation are in English
- Tests pass for new features