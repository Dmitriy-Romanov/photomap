# Exif Parser Test Tool 🕵️‍♂️

This utility is designed to debug and verify the EXIF parsing logic of the main `photomap` application. It compares our internal parsing logic against a robust "reference" library (`rexif`) to identify files where we fail to extract GPS data but should be able to.

## Purpose

If a user reports that photos in a specific folder are not appearing on the map (i.e., GPS data is missing), use this tool to diagnose the issue.

1.  **Compare**: It scans a folder and tries to extract GPS using both "Our" logic (ported from `photomap`) and "Reference" logic (`rexif`).
2.  **Log Failures**: If "Reference" finds GPS but "Our" logic doesn't, the file path is logged to `failures.txt`.
3.  **Analyze**: These "failure" files are candidates for debugging. We can then inspect them to understand why our parser is failing (e.g., unsupported format, weird header, etc.).

## How to Use

1.  **Build & Run**:
    ```bash
    cd exif_parser_test
    cargo run
    ```
2.  **Select Folder**: A native dialog will appear. Select the folder containing the problematic photos.
3.  **Check Results**:
    *   The tool will print progress.
    *   After completion, check `failures.txt` in the `exif_parser_test` directory.
    *   **If `failures.txt` is empty**: Our parser is working as well as the reference. The issue might be that the files simply don't have GPS data.
    *   **If `failures.txt` has entries**: These files are the key. Copy one of them and analyze it (e.g., check headers, try other parsers).

## Project Structure

*   `src/main.rs`: Contains the logic.
    *   `extract_gps_our`: **Must match the logic in `photomap/src/processing.rs` and `photomap/src/exif_parser/`**. If you update the main app's parser, update this function too to keep them in sync!
    *   `extract_gps_ref`: Uses `rexif` crate as a ground truth.
*   `failures.txt`: Generated report (ignored in git).

## Known Issues & Fixes

*   **Xiaomi HEIC Bug**: We discovered that some Xiaomi phones save JPEG files with `.heic` extension. This tool helped identify that. The fix (checking for `FF D8` signature) is implemented in both this tool and the main `photomap` app.
