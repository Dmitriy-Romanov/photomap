# Project Map

This document provides a comprehensive overview of the `photomap` project structure, modules, and their interactions.

## Directory Structure

```
photomap/
├── src/
│   ├── constants.rs
│   ├── database.rs
│   ├── exif_parser/
│   │   ├── generic.rs
│   │   ├── gps_parser.rs
│   │   ├── heic.rs
│   │   ├── jpeg.rs
│   │   └── mod.rs
│   ├── geocoding.rs
│   ├── geodata.bin.gz
│   ├── image_processing.rs
│   ├── main.rs
│   ├── process_manager.rs
│   ├── processing.rs
│   ├── server/
│   │   ├── events.rs
│   │   ├── handlers.rs
│   │   ├── mod.rs
│   │   └── state.rs
│   ├── settings.rs
│   └── utils.rs
├── frontend/
│   ├── index.html
│   ├── script.js
│   └── style.css
├── tools/
│   ├── exif_parser_test/
│   └── geodata_builder/
├── Cargo.toml
└── ...
```

## Module Overview

### `main.rs`

*   **Purpose:** The entry point of the application.
*   **Responsibilities:**
    *   Initializes the application (logging, single instance check).
    *   Initializes the database (`database.rs`).
    *   Loads settings (`settings.rs`).
    *   Starts the web server (`server/mod.rs`).
    *   Loads configured folders and reuses the binary cache when folder paths match.
    *   Processes configured folders on startup when the cache is missing or invalid.

### `server/`

*   **Purpose:** The web server module, built with `axum`.
*   **`mod.rs`:** The root of the server module.
    *   Declares the other server sub-modules.
    *   Contains the `create_app` function, which builds the `axum` router and defines the API routes.
    *   Contains the `start_server` function, which starts the web server.
*   **`handlers.rs`:** Contains all the `axum` handler functions for the API endpoints.
    *   `get_all_photos`: Returns a list of all photos with their metadata.
    *   `serve_processed_image`: Serves dynamically resized images (markers, thumbnails).
    *   `convert_heic`: Converts HEIC images to JPEG on the fly.
    *   `get_settings`, `set_folder`, `update_settings`: Handles application settings.
    *   `reprocess_photos`, `initiate_processing`: Triggers photo processing.
    *   `processing_events_stream`: Provides real-time updates on photo processing via Server-Sent Events (SSE).
    *   `shutdown_app`: Gracefully shuts down the server.
*   **`state.rs`:** Defines the `AppState` struct, which is shared across all `axum` handlers. It contains the database connection, application settings, and the SSE event sender.
*   **`events.rs`:** Defines the `ProcessingEvent` and `ProcessingData` structs used for SSE.

### `processing.rs`

*   **Purpose:** Contains the core logic for processing photos.
*   **Responsibilities:**
    *   Scans configured photo directories.
    *   Extracts EXIF metadata from photos using the `exif_parser` module.
    *   Saves photo metadata to the database.

### `database.rs`

*   **Purpose:** Manages the In-Memory database and persistence operations.
*   **Responsibilities:**
    *   Stores photo metadata in memory.
    *   Provides functions to insert, query, clear, save, and load photo metadata.
    *   Persists the cache as `photos_v1.bin`.

### `exif_parser/`

*   **Purpose:** Extracts EXIF metadata from various image formats.
*   **`mod.rs`:** The root of the `exif_parser` module. Declares and exports functions from the sub-modules.
*   **`heic.rs`:** Contains `extract_metadata_from_heic` for parsing HEIC files using the `libheif-rs` library.
*   **`jpeg.rs`:** Contains `extract_metadata_from_jpeg` for parsing JPEG files using the `kamadak-exif` library.
*   **`gps_parser.rs`:** Contains a low-level GPS parser used as a fallback for malformed JPEG EXIF.
*   **`generic.rs`:** Contains generic EXIF parsing functions like `get_gps_coord` and `get_datetime_from_exif`, and `apply_exif_orientation`.

### `image_processing.rs`

*   **Purpose:** Handles image manipulation tasks.
*   **Responsibilities:**
    *   `create_scaled_image_in_memory`: Creates resized versions of images (markers, thumbnails).
        *   **Optimization**: Uses `turbojpeg` for fast JPEG scaling and `Triangle` filter for quality/speed balance.
    *   `convert_heic_to_jpeg`: Converts HEIC images to JPEG.

### `settings.rs`

*   **Purpose:** Manages application settings.
*   **Responsibilities:**
    *   Loads settings from an `.ini` file.
    *   Saves settings to an `.ini` file.
*   The `Settings` struct is shared across the application using an `Arc<Mutex<>>`.

### `geocoding.rs`

*   **Purpose:** Provides offline reverse geocoding.
*   **Responsibilities:**
    *   Loads the embedded GeoNames data from `geodata.bin.gz`.
    *   Resolves coordinates to city/country labels without network calls.

### `process_manager.rs`

*   **Purpose:** Ensures that only a single instance of the application is running.

### `utils.rs`

*   **Purpose:** Contains cross-platform utility functions.
*   **Responsibilities:**
    *   Resolves app data and config paths.
    *   Provides native folder selection dialogs for macOS, Windows, and Linux.
    *   Opens the browser on startup when enabled.

### `constants.rs`

*   **Purpose:** Defines constants used throughout the application.
    *   **Update**: `POPUP_SIZE` increased to 1400 for HiDPI.

## Frontend (`frontend/`)

*   **`index.html`**:
    *   Structure of the application.
    *   Includes map container, floating info window, controls.
    *   Includes year range inputs, visualization toggles, and the "Close map" button.
*   **`script.js`**:
    *   `initializeMap`: Sets up Leaflet map and clusters.
    *   `processFolder`: Handles processing workflow.
    *   `filterMarkers`: Filters photos by year range.
    *   `shutdownApp`: Calls shutdown API and closes the window.
*   **`style.css`**:
    *   Styling for map, info window, and controls.
    *   Handles responsive design and animations.

## Tools (`tools/`)

*   **`geodata_builder/`**:
    *   Build-time helper for regenerating `src/geodata.bin.gz` from a GeoNames `cities1000.txt` TSV file.
    *   Kept outside the main Cargo package so the normal PhotoMap build still produces only the application binary.
*   **`exif_parser_test/`**:
    *   Standalone diagnostic tool for EXIF/GPS parser validation.
    *   Kept outside the main Cargo package because it is not part of the PhotoMap runtime.

## Data Flow

1.  **Application Start:** `main.rs` initializes the database and settings.
2.  **Web Server Start:** `main.rs` starts the web server by calling `server::start_server`.
3.  **API Request:** The frontend sends an API request to the server.
4.  **Routing:** `server/mod.rs` routes the request to the appropriate handler in `server/handlers.rs`.
5.  **Handler Logic:** The handler function processes the request, interacting with the database (`database.rs`), settings (`settings.rs`), or triggering photo processing (`processing.rs`).
6.  **Photo Processing:**
    *   `processing.rs` scans the photo directory.
    *   For each photo, it calls the appropriate function from `exif_parser` to extract metadata.
    *   The metadata is saved to the database via `database.rs`.
    *   During processing, events are sent to the frontend via SSE (`server/events.rs`).
7.  **Image Serving:**
    *   When the frontend requests an image (`/api/marker/*` or `/api/thumbnail/*`), the `serve_processed_image` handler in `server/handlers.rs` is called.
    *   This handler uses `image_processing.rs` to resize the image and sends it back to the frontend.
    *   HEIC images are converted to JPEG on the fly by `image_processing.rs`.
8.  **Shutdown Flow:**
    *   User clicks "Close map".
    *   Frontend calls `/api/shutdown`.
    *   Server initiates graceful shutdown.
    *   Frontend closes the browser tab.
