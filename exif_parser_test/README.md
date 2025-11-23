# Exif Parser Test Tool 🕵️‍♂️

This utility is designed to debug and verify the EXIF parsing logic of the main `photomap` application. It compares our internal parsing logic against the industry-standard **`exiftool`** CLI to identify files where we fail to extract GPS data but should be able to.

## Purpose

If photos in a specific folder are not appearing on the map (i.e., GPS data is missing), use this tool to diagnose the issue.

1.  **Compare**: It scans a folder and tries to extract GPS using "Our" logic (identical to `photomap`) and "Exiftool" (gold standard).
2.  **Log Failures**: If exiftool finds GPS but our logic doesn't, the file path is logged to `failures.txt`.
3.  **Analyze**: These "failure" files are candidates for debugging. We can then inspect them to understand why our parser is failing.
4.  **Auto-copy**: Failed files are automatically copied to `JPG for checks/` for easy inspection.
5.  **Accuracy Check**: Compares coordinates from our parser with exiftool to detect parsing errors (not just missing GPS).

## Download Pre-built Binary (Windows)

**No compilation needed!** Download from GitHub Actions:

1. Go to [Actions tab](../../actions/workflows/build-windows.yml)
2. Click on the latest successful run
3. Download `exif_parser_test_windows-x64` artifact
4. Extract `exif_parser_test_windows-x64.exe`
5. Double-click to run!

> **Note:** You need `exiftool` installed. Download from [exiftool.org](https://exiftool.org/) and add to PATH.

## Build from Source

### Requirements
- Rust toolchain
- `exiftool` CLI (install via Homebrew on macOS/Linux, or from exiftool.org on Windows)
- vcpkg (for libheif on Windows)

### Build & Run
```bash
cd exif_parser_test
cargo build --release
cargo run --release
```

## How to Use

1.  **Run the tool**
2.  **Select Folder**: A native dialog will appear. Select the folder containing the photos to test.
3.  **Wait**: The tool will process all images and show progress.
4.  **Check Results**:
    *   The tool will print progress for each file.
    *   After completion, check `failures.txt` and `accuracy_issues.txt`.
    *   **If both files are empty**: Our parser works perfectly! ✅
    *   **If `failures.txt` has entries**: GPS data exists but we didn't find it → parser coverage issue
    *   **If `accuracy_issues.txt` has entries**: We found GPS but coordinates don't match exiftool → parser accuracy issue

## Output Files

- **`failures.txt`**: Files where exiftool found GPS but our parser didn't (missing data)
- **`accuracy_issues.txt`**: Files where our parser returned different coordinates than exiftool (wrong data)
- **`JPG for checks/`**: Copies of failed files for manual inspection

## Dependencies

**Minimal set - essential dependencies:**
- `walkdir` - Directory traversal
- `rfd` - Native file picker dialog
- `anyhow` - Error handling
- `kamadak-exif` - Our EXIF parser (same as PhotoMap)
- `libheif-rs` - HEIC/HEIF support (same as PhotoMap)
- **`exiftool`** (CLI) - Gold standard reference (must be installed separately)

## Project Structure

*   `src/main.rs`: Contains the logic.
    *   `extract_gps_our`: **Identical to PhotoMap's GPS parsing logic**. 
    *   `extract_gps_exiftool`: Uses `exiftool` CLI as ground truth (99.99% accuracy).
*   `failures.txt`: Generated report of files with missing GPS.
*   `accuracy_issues.txt`: Generated report of files with coordinate mismatches.
*   `JPG for checks/`: Auto-populated with copies of failed files.

## Parser Features

Our parser (`extract_gps_our`) now includes:
- ✅ Checks `In::PRIMARY` IFD first (fast path for most cameras)
- ✅ Falls back to iterating ALL EXIF fields if not found
- ✅ Finds GPS data in any IFD location (fixes Samsung SM-G780G and similar)
- ✅ Handles partial EXIF with `continue_on_error(true)`
- ✅ Supports non-standard EXIF structures (e.g., Lightroom-processed)
- ✅ Full HEIC/HEIF/AVIF support via libheif-rs

## Validation Modes

**1. Coverage Check (Missing GPS):**
```
Our parser: ✗ FAILED
exiftool: ✓ (48.8566, 2.3522)
→ failures.txt
```

**2. Accuracy Check (Wrong Coordinates):**
```
Our parser: (48.8566, 2.3522)
exiftool:   (48.8570, 2.3525)
Difference: Δlat=0.0004°, Δlon=0.0003°
→ accuracy_issues.txt (if > 0.0001° / ~11m)
```

## Known Fixes

*   **Samsung SM-G780G GPS**: Fixed by searching all IFD fields, not just PRIMARY
*   **Xiaomi HEIC Bug**: HEIC files that are actually JPEG - handled via FF D8 signature check
*   **Lightroom EXIF**: Handled via `continue_on_error(true)`

## Notes

-   Test tool parser is **100% identical** to PhotoMap: JPEG, HEIC, HEIF, AVIF all supported.
-   Tolerance for coordinate differences: **0.0001°** (~11 meters)
-   For large datasets (100k+ files), this test may take hours as it calls exiftool for every file.
