# Project: PhotoMap

## Project Overview

PhotoMap is a cross-platform desktop application that allows users to visualize their photos on an interactive map. It consists of a Rust backend that acts as a local web server and a vanilla HTML/CSS/JavaScript frontend for the user interface.

The backend, built with `actix-web`, scans user-specified directories for image files (including JPG and HEIC/HEIF). It parses EXIF metadata to extract GPS coordinates and creation dates. This metadata is stored in an in-memory cache which can be persisted to disk (`photos.bin`) to speed up subsequent launches. The server exposes a REST API for the frontend to fetch photo data and serves the static frontend files.

The frontend is a single-page application using Leaflet.js to display an interactive map. It communicates with the backend to fetch photo metadata and displays markers on the map. It features:
- Marker clustering for handling large numbers of photos.
- A gallery view to browse photos within a cluster.
- UI controls for selecting photo directories, filtering by year, and managing application settings.

The project is designed to be a single, self-contained executable.

## Building and Running

The project is built using Cargo, the Rust build tool.

### Dependencies

- **Rust toolchain:** Can be installed via `rustup`.
- **`libheif`:** Required for HEIC/HEIF image support. Installation varies by OS (see `.github/workflows/ci.yml` for detailed setup).

### Key Commands

- **Check & Lint:**
  ```bash
  cargo clippy
  ```

- **Run Tests:**
  ```bash
  cargo test
  ```

- **Build (Debug):**
  ```bash
  cargo build
  ```

- **Build (Release):** For a smaller, optimized executable.
  ```bash
  cargo build --release
  ```

- **Run the Application:**
  ```bash
  cargo run --release
  ```
  After running, the application starts a web server at `http://127.0.0.1:3001` and should open this URL in your default web browser automatically.

## Development Conventions

- **Code Style:** The codebase follows standard Rust formatting, enforced by `rustfmt`. `cargo clippy` is used for linting.
- **Database:** The application uses an in-memory data structure (`Vec<PhotoMetadata>`) as its primary "database". This data is cached to a binary file in the application's data directory to avoid reprocessing photos on every launch.
- **API:** A simple RESTful API is used for communication between the frontend and backend.
- **Testing:** The CI pipeline runs `cargo test`, suggesting that unit and integration tests are part of the development process.
- **Cross-Platform Builds:** The CI configuration includes build jobs for Linux, macOS, and Windows, indicating a focus on cross-platform compatibility. Special care is taken to link `libheif` statically where possible.

## EXIF Parser Test Utility

The project includes a separate, crucial utility located in the `exif_parser_test/` directory. This is a command-line tool designed to act as a "test bench" to validate the accuracy and coverage of the GPS data parsing logic used in the main application.

### Purpose and Methodology

- **Validation:** It rigorously checks the PhotoMap's Rust-based EXIF parsing code against the output of the industry-standard `exiftool`.
- **Identical Code:** It uses the same versions of `kamadak-exif`, `libheif-rs`, and the same custom parsing logic as the main application to ensure the test is representative.
- **Reporting:** When discrepancies are found, the tool generates reports:
    - `failures.txt`: Lists images where `exiftool` found GPS data but the app's parser did not.
    - `accuracy_issues.txt`: Lists images where the extracted GPS coordinates do not match `exiftool`'s output.
    - `JPG for checks/`: A directory containing copies of problematic files for easier debugging.

### Workflow

This tool is central to the development workflow for the EXIF parsing module:
1.  Run the test utility on a large collection of photos.
2.  Analyze the generated reports to identify bugs (either missed GPS data or inaccurate coordinates).
3.  Debug and fix the parsing logic within the `exif_parser_test` tool itself.
4.  Once verified, port the corrected code back into the main PhotoMap application's `src/exif_parser/` module.

This approach ensures the core data extraction logic is robust and reliable before being integrated into the main application. This utility has its own dependencies and must be built and run from its subdirectory. It has a critical runtime dependency on `exiftool`, which must be installed and available in the system's PATH or placed next to the executable.

## Working Modes (Antigravity IDE)

### PLANNING Mode
**Purpose:** Research, discussion, and planning phase.

**Rules:**
- **NEVER change production code** (src/, frontend/, etc.)
- **Only modify:** `.md` files (implementation_plan.md, task.md, etc.)
- This is the **"time to talk about how we implement"** phase
- Discuss approaches, create plans, ask clarifying questions
- Wait for explicit command to proceed (e.g., "делаем", "go ahead", "implement it")

**Transition to EXECUTION:**
- User explicitly says to proceed with implementation
- Create/update `implementation_plan.md` and get approval first

### EXECUTION Mode  
**Purpose:** Active implementation phase.

**Rules:**
- **Work with minimal questions** - implementation is approved
- Make code changes, run builds, test changes
- Update `task.md` as work progresses
- Complete the approved plan

**Important:** If UI always shows "PLANNING" mode, ignore it - follow user's explicit instructions about which mode to be in.
