# Changelog

All notable changes to PhotoMap will be documented in this file.

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
