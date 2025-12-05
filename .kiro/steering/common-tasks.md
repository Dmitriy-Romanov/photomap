---
inclusion: manual
---

# Common Development Tasks

This guide covers frequent development scenarios and how to approach them.

## Adding Support for New Image Format

1. Add file extension to filter in `processing.rs` (`process_file_to_metadata`)
2. Create new parser in `src/exif_parser/` if needed
3. Update `is_heif` logic if format requires special handling
4. Test with sample files in `exif_parser_test/`

## Modifying Database Schema

1. Update `PhotoMetadata` struct in `database.rs`
2. Increment cache version number in `CachedDatabase`
3. Old caches will auto-delete on version mismatch
4. Update API response in `ImageMetadata` if needed

## Adding New API Endpoint

1. Add handler function in `server/handlers.rs`
2. Register route in `server/mod.rs` (`create_app`)
3. Update frontend API calls in `script.js` (use centralized `API` object)
4. Document endpoint in `tech.md` if public-facing

## Performance Optimization Checklist

- [ ] Use `rayon` for CPU-bound parallel work
- [ ] Batch database operations (avoid per-item locks)
- [ ] Consider lazy initialization for expensive resources
- [ ] Profile with `cargo build --release` (debug builds are 10x slower)
- [ ] Check binary size impact (`ls -lh target/release/photomap_processor`)

## Debugging EXIF Issues

1. Use `exif_parser_test` tool in `exif_parser_test/` directory
2. Copy problematic files to `JPG for checks/` folder
3. Run: `cd exif_parser_test && cargo run --release`
4. Check `failures.txt` and `accuracy_issues.txt` for details
5. Add fallback logic in appropriate parser

## Testing Changes

```bash
# Quick compile check
cargo check

# Run with detailed logs
RUST_LOG=debug cargo run

# Test with specific photo folder
# (modify settings.ini or use web UI)

# Check for warnings
cargo clippy

# Format code
cargo fmt
```

## Common Gotchas

- **Frontend changes**: Assets are embedded at compile time - rebuild after changes
- **Cache issues**: Delete `~/.photomap/photos_v1.bin` to force reprocessing
- **Port conflicts**: App kills existing instances, but check for orphaned processes
- **HEIC on Linux**: May need `libheif-dev` package installed
- **Path separators**: Always use `/` internally, convert `\` on Windows input

## Version Bump Workflow

1. Update version in `Cargo.toml`
2. Update `VERSION` constant if used elsewhere
3. Add entry to `CHANGELOG.md`
4. Update `README.md` version history
5. Build release: `cargo build --release`
6. Test binary thoroughly before tagging

## Frontend Development

- Frontend files are in `frontend/` directory
- Embedded via `rust-embed` at compile time
- Changes require full rebuild (`cargo build`)
- Use browser DevTools for debugging
- API endpoints documented in `script.js` `API` object
