# Changelog

All notable changes to PhotoMap will be documented in this file.

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
