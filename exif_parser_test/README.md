# Exif Parser Test Tool 🕵️‍♂️

This utility is designed to debug and verify the EXIF parsing logic of the main `photomap` application. It compares our internal parsing logic against the industry-standard **`exiftool`** CLI to identify files where we fail to extract GPS data but should be able to.

## Purpose

If photos in a specific folder are not appearing on the map (i.e., GPS data is missing), use this tool to diagnose the issue.

1.  **Compare**: It scans a folder and tries to extract GPS using "Our" logic (identical to `photomap`) and "Exiftool" (gold standard).
2.  **Log Failures**: If exiftool finds GPS but our logic doesn't, the file path is logged to `failures.txt`.
3.  **Analyze**: These "failure" files are candidates for debugging. We can then inspect them to understand why our parser is failing.
4.  **Auto-copy**: Failed files are automatically copied to `JPG for checks/` for easy inspection.

## How to Use

1.  **Build & Run**:
    ```bash
    cd exif_parser_test
    cargo run --release
    ```
2.  **Select Folder**: A native dialog will appear. Select the folder containing the photos to test.
3.  **Check Results**:
    *   The tool will print progress for each file.
    *   After completion, check `failures.txt` in the `exif_parser_test` directory.
    *   **If `failures.txt` is empty**: Our parser is working as well as exiftool. The original issue is that files simply don't have GPS data.
    *   **If `failures.txt` has entries**: These files need investigation. Check `JPG for checks/` folder for copies.

## Dependencies

**Minimal set - no bloat!**
- `walkdir` - Directory traversal
- `rfd` - Native file picker dialog
- `anyhow` - Error handling
- `kamadak-exif` - Our EXIF parser (same as PhotoMap)
- **`exiftool`** (CLI) - Gold standard reference (installed via Homebrew)

**Removed** (not needed in test tool):
- ~~`rexif`~~ - Redundant reference parser
- ~~`libheif-rs`~~ - HEIC support (PhotoMap handles this, test focuses on JPEG)

## Project Structure

*   `src/main.rs`: Contains the logic.
    *   `extract_gps_our`: **Identical to PhotoMap's GPS parsing logic**. If you update the main app's parser, this stays in sync automatically as both use the same `kamadak-exif` approach.
    *   `extract_gps_exiftool`: Uses `exiftool` CLI as ground truth (99.99% accuracy).
*   `failures.txt`: Generated report of problematic files.
*   `JPG for checks/`: Auto-populated with copies of failed files.

## Parser Features

Our parser (`extract_gps_our`) now includes:
- ✅ Checks `In::PRIMARY` IFD first (fast path for most cameras)
- ✅ Falls back to iterating ALL EXIF fields if not found
- ✅ Finds GPS data in any IFD location (fixes Samsung SM-G780G and similar)
- ✅ Handles partial EXIF with `continue_on_error(true)`
- ✅ Supports non-standard EXIF structures (e.g., Lightroom-processed)

## Known Fixes

*   **Samsung SM-G780G GPS**: Fixed by searching all IFD fields, not just PRIMARY
*   **Xiaomi HEIC Bug**: Not applicable (test tool skips HEIC files)
*   **Lightroom EXIF**: Handled via `continue_on_error(true)`

## Notes

-   **HEIC/HEIF/AVIF files are skipped** in this test tool. PhotoMap has full support via `libheif-rs`, but for testing we focus on JPEG as it's the most common format and easier to debug.
-   Test tool uses **release mode** (`cargo run --release`) for production-like performance.
