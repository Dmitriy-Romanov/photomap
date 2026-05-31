# Changelog

All notable changes to PhotoMap will be documented in this file.

## [Unreleased] - 2026-05-31

### Fixed
- **Cache Compatibility Regression**: Restored legacy `bincode` fixed-integer decoding while keeping bounded reads, so `photos_v1.bin` and embedded GeoNames data deserialize correctly instead of forcing unnecessary full rescans.
- **Windows Path Normalization**: Normalized configured folders, cached file paths, processing logs, tooltip paths, reveal-file payloads, and image-decoding paths to native Windows separators while keeping relative URL paths browser-safe.
- **Photo URL Encoding**: Encoded marker, thumbnail, gallery, popup, and HEIC conversion URL paths per segment so spaces, Cyrillic names, and mixed slash input resolve consistently.
- **Lazy Popup Image Loading**: Changed Leaflet popups to build content only when opened, preventing thousands of background popup image conversions for large collections.
- **Map Marker Load Behavior**: Restored lightweight marker image usage for the map instead of requesting larger thumbnails for every marker.
- **Frontend Injection Hardening**: Replaced popup/gallery metadata template-string HTML with DOM node construction and `textContent`, and assigned image `src`/fallback URLs via DOM properties.

## [0.12.0] - 2026-05-30

### Added
- **Dynamic CLI Port Configuration**: Added zero-dependency command-line parsing for `-p` / `--port <port>` and `-h` / `--help` on startup, allowing flexible custom binding.
- **Robust Cache Size Limits**: Added strict size limits (`50MB` for database cache and `20MB` for embedded GeoNames) to `bincode::deserialize_from` calls to avoid potential OOM or panic crashes on corrupt files.
- **Automatic HEIC Temp File Cleanup**: Implemented a `TempFileGuard` using RAII drop pattern to guarantee temporary files or symlinks are removed from disk on failures or panic.
- **Type-Safe GPS Missing Errors**: Defined custom `ExifError` enum using `thiserror` to handle missing GPS coordinates type-safely via downcasting, removing error-prone string checks.
- **Directory Traversal Warning Logs**: Replaced silent error skips during directory scanning with explicit logging of traversal and read warnings to standard error.

### Changed
- **Buffered Processing Events**: Reworked processing event delivery from direct broadcast into an `mpsc` queue with broadcast fan-out, reducing lost progress/completion events when the frontend connects late.
- **Indexed In-Memory Database**: Replaced vector-based metadata storage and O(n²) deduplication with a `HashMap<String, PhotoMetadata>` keyed by relative path.
- **O(1) Image Lookup**: Added direct `get_photo_by_relative_path()` lookup for marker, thumbnail, gallery, popup, and original photo routes instead of cloning and scanning the full database.
- **Async Boundary Cleanup**: Moved CPU-heavy resizing, folder selection, and reprocessing work to blocking threads or `spawn_blocking` so the Tokio runtime stays responsive.
- **Async Settings Lock**: Replaced the settings `std::sync::Mutex` with `tokio::sync::Mutex` for async handlers.
- **HEIC Decode Boundary**: Kept HEIC conversion behind the blocking image-processing path to avoid decoding work on async runtime workers.
- **Single SSE Keepalive Source**: Removed the custom heartbeat loop and kept Axum's SSE `KeepAlive` as the only heartbeat mechanism.
- **Async File Serving**: Switched original-photo reads in `serve_photo` to `tokio::fs::read`.

### Fixed
- **HEIC Non-UTF-8 Paths**: HEIC conversion now reports graceful errors for paths that cannot be represented safely instead of panicking.
- **Corrupt/Absent GeoData**: Reverse geocoding initialization now falls back to disabled geocoding instead of panicking when embedded data is unavailable or invalid.
- **Settings Toggle Endpoint**: Frontend settings toggles now post to `/api/update_settings`, matching the backend route.
- **Response Builder Errors**: Response construction failures are propagated as `500` responses instead of unwrapping.
- **Duplicate HTML ID**: Removed duplicate `exp-year-range-label` markup.
- **Popup Escaping**: Escaped popup filenames and file paths to prevent HTML/attribute injection.
- **Year Range Label**: Replaced the em dash with an ASCII hyphen and adjusted label sizing.
- **Non-ASCII Marker URLs**: Encoded marker/thumbnail/popup path segments so Cyrillic and other non-ASCII folder names load correctly.

### Security
- **Command Injection Prevention**: Fixed command injection vulnerability in `reveal_file` on Windows by executing `explorer.exe` directly instead of using shell execution (`cmd /C start`).
- **Localhost-Only CORS Restrict**: Restricted `CorsLayer` origins to `localhost` and `127.0.0.1`, blocking malicious websites from querying private local photo EXIF or file maps.

## [0.11.0] - 2026-05-19

### Changed
- **GeoData size reduction**: Rebuilt `src/geodata.bin.gz` from GeoNames `cities5000.txt` and removed the unused `admin1` field from embedded geodata records.
- **Smaller binary**: Embedded geodata reduced from ~3.1MB to ~1.2MB, reducing the release binary from ~4.8MB to ~2.9MB on macOS arm64.
- **Tooling cleanup**: Restored `tools/geodata_builder`, moved `exif_parser_test` under `tools/`, and split `utils.rs` into focused modules.

## [0.10.2] - 2026-04-19

### Fixed
- **EXIF Parser Improvements**: Fixed file duplication in jpeg.rs by caching datetime from first EXIF read
- **Float Validation**: Added NaN/Infinity validation in GPS coordinate parsers (gps_parser.rs, generic.rs)
- **Documentation**: Translated all markdown files to English (CLAUDE.md, PROJECT_MAP.md, etc.)
- **Cleanup**: Removed temporary documentation files (.#continue.sh)

## [0.9.9] - 2026-03-26

### Changed
- **Dependency Cleanup Round 2**: Removed 4 more unnecessary dependencies
  - Replaced `rust-embed` with std `include_bytes!` for 3 frontend files
  - Replaced `tokio-stream` with 15-line custom `ReceiverStream` adapter
  - Replaced `tracing` + `tracing-subscriber` with `println!`/`eprintln!`
  - Added `futures-core` (already a transitive dependency of tokio)
- **Binary size**: Reduced from 5.7MB to 5.1MB (-11%)

### Fixed
- **Code Cleanup**: Removed redundant variables and unified duplicate code
  - Removed unused `_min_dim` variable in image processing
  - Unified image format validation in single `constants.rs` function
  - Fixed `lon`/`lng` inconsistency - now consistently uses `lng` everywhere
  - Removed redundant `gps_count` from statistics (always equaled `processed_count`)
  - Fixed `serve_photo` handler to work with multiple folders (was using only `folders[0]`)
- **Net change**: 67 lines added, 67 lines removed (pure refactoring, no functionality loss)

## [0.9.8] - 2026-03-24

### Changed
- **Dependency Cleanup**: Removed 3 unnecessary external dependencies in favor of std library implementations
  - Replaced `ignore` crate with custom `walk_dir()` using `std::fs::read_dir`
  - Replaced `chrono` crate with manual datetime parsing (`parse_exif_datetime()`)
  - Replaced `kdtree` crate with simple linear search (fast enough for 163k cities)
- **Code Simplification**: More direct std-lib usage, less external magic
- **Build Performance**: Faster compilation due to fewer dependencies

### Technical Details
- `walk_dir()`: ~20 lines of iterative directory traversal (prevents stack overflow on deep trees)
- `parse_exif_datetime()`: ~15 lines for EXIF datetime format parsing
- Linear geocoding search: ~25 lines, performs ~1-2ms for 163k cities

## [0.9.7] - 2026-03-22

### Security
- **Backend Dependencies Update**: Performed full `cargo update` to address vulnerabilities
  - Patched `image` crate (WebP security fix)
  - Updated `libheif-rs` and `chrono` to address memory safety and data handling issues
- **Frontend Dependencies Pinning**: Replaced `@latest` tags with fixed versions for Leaflet and MarkerCluster to prevent supply chain attacks

### Changed
- **CI/CD Infrastructure**:
  - Upgraded GitHub Actions to `@v4`
  - Opted-in to Node.js 24 environment for all workflows
  - Improved caching strategy for faster builds

## [0.9.6] - 2025-11-28

### Added
- **Offline Reverse Geocoding**: Implemented city/country display for photo locations
  - Embedded GeoNames database (140k+ cities) compressed into the executable
  - KD-Tree spatial index for fast nearest-neighbor lookups
  - Location displayed as "📍 City, Country" alongside photo date in popups
  - Fully offline - no external API calls required
  - New backend module `src/geocoding.rs` with lazy initialization
  - Extended `ImageMetadata` struct with optional `location` field

### Changed
- **Popup UI Unification**: Standardized popup layout between map markers and gallery
  - Unified CSS classes (`.popup-filename`, `.popup-metadata`)
  - Removed duplicate styles and wrapper divs
  - Consistent font sizing and alignment
  - Metadata line format: "📍 Location  📅 DD-MM-YYYY HH:MM"

### Fixed
- **Gallery Popup Layout**: Fixed incorrect inline display of metadata in gallery detail view
- **Coordinate Tooltips**: Removed unwanted tooltips from map coordinate display

### Removed
- **Build Tools**: Removed `tools/geodata_builder` directory (one-time use for database generation)

## [0.9.5] - 2025-11-28

### Added
- **SVG Sprite System**: Implemented SVG sprites in `index.html` for cleaner code and better icon management.
- **JSDoc Documentation**: Comprehensive documentation added to `script.js` for better maintainability.
- **Centralized API**: Refactored `script.js` to use a centralized `API` constant for all endpoints.

### Changed
- **Path Handling**: Refactored `formatPhotoData` to use event delegation and `data-full-path` attributes, removing inline `onclick` handlers and improving security/reliability for Windows paths.
- **UI Styling**: Refactored inline styles to CSS utility classes (`.text-reset`, `.w-19`, `.w-35`).
- **Panel Layout**: Adjusted experimental panel width to 540px and optimized column widths in Row 2.
- **Font Consistency**: Fixed font weight issues in panel labels.

### Fixed
- Typo in photo popup ("Photo shooted" -> "Photo taken").

## [0.9.4] - 2025-11-27

### Changed
- **Frontend Architecture**: Major refactoring of `script.js` into logical controllers (Data Service, Map Controller, UI Controller).
- **Windows Integration**: Improved "Reveal File" functionality to correctly focus Explorer and select the file.

### Fixed
- **Path Escaping**: Fixed issues with backslash escaping in file paths on Windows.
- **Popup Styling**: Unified styling for map popups.

## [0.9.3] - 2025-11-27

### Added
- **Multi-folder support**: Select and process up to 5 photo folders simultaneously
  - Native folder selection dialogs for all platforms (macOS, Windows, Linux)
  - Settings persistence: Stores multiple folder paths in INI file
  - Frontend displays "Multiple folders (N)" when multiple folders selected
  - All selected folders processed on startup

### Changed
- **Cache system v1**: Improved cache with versioning and auto-cleanup
  - Cache stores folder paths array instead of single path
  - Automatic cleanup of incompatible/outdated cache files
  - 100% folder path matching required for cache hit
  - Prevents memory allocation crashes from incompatible cache formats

- **Database clearing**: Optimized to clear once before processing all folders
  - Prevents data loss when processing multiple folders
  - All photos from all folders accumulated correctly

- **Windows folder selection UX**: Improved dialog prompts
  - First dialog: "Select folder 1 (max 5)"
  - Subsequent dialogs: "Add folder 2? (Cancel = Done)"
  - Cancel anytime to finish selection

### Removed
- Legacy single-folder functions (`process_photos_into_database`, `select_folder_native`)
- All dead code warnings eliminated

### Fixed
- Database clearing issue where second folder would delete first folder's photos
- Frontend not loading saved folders on startup
- SSE events not sent after reprocessing
- Cache compatibility crashes from format changes

## [0.9.2] - 2025-11-24

### Changed
- Code quality improvements
- Frontend refactoring for better maintainability

## Previous versions
See git history for older releases.
