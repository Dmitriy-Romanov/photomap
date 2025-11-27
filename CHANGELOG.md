# Changelog

All notable changes to PhotoMap will be documented in this file.

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
