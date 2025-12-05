# Product Overview

PhotoMap Processor is a high-performance photo mapping application that extracts GPS coordinates from photo EXIF metadata and displays them on an interactive web-based map.

**Target Users**: Photographers, travelers, and anyone with large photo collections who wants to visualize where their photos were taken.

**Performance Focus**: The application prioritizes speed and low memory usage, using parallel processing and aggressive binary size optimizations.

## Core Features

- **Parallel Photo Processing**: Scans photo directories and extracts GPS/EXIF metadata using Rayon for parallel processing
- **Multi-Format Support**: Handles JPEG, HEIC/HEIF, and AVIF image formats
- **In-Memory Database**: Fast photo metadata storage with binary cache persistence
- **On-Demand Image Processing**: Dynamic thumbnail and marker generation via HTTP API
- **Offline Reverse Geocoding**: Embedded GeoNames database (140k+ cities) with KD-Tree spatial indexing for instant location lookups
- **Interactive Web Interface**: Leaflet-based map with clustering, heatmaps, and route visualization
- **Real-Time Updates**: Server-Sent Events (SSE) for processing progress

## Architecture

- **Backend**: Rust-based HTTP server (Axum) with embedded web assets
- **Frontend**: Vanilla JavaScript with Leaflet.js for mapping
- **Storage**: In-memory database with bincode serialization for persistence
- **Processing**: Multi-threaded photo scanning and EXIF extraction

## Key Use Cases

1. Visualize photo collections on a map
2. Browse photos by geographic location
3. Filter photos by year range
4. View photo density heatmaps and travel routes
5. Process multiple photo folders simultaneously (up to 5)
