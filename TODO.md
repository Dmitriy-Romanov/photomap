# PhotoMap TODO

Living list of known remaining work. Completed v0.12.0 fixes were removed after being documented in `CHANGELOG.md`.

## High Priority

| # | Issue | File / Area | Notes | Status |
|---|---|---|---|:---:|
| 1 | HEIC conversion still blocks async handler | `src/server/handlers.rs` | Wrapped `/convert-heic` work in `tokio::task::spawn_blocking`. | ✅ DONE |
| 2 | Frontend metadata HTML injection risk | `frontend/script.js` | Escape `photo.datetime` and `photo.location` before inserting popup/gallery metadata via `innerHTML`. | ⬜ |
| 3 | Image URL attributes inserted without attribute escaping | `frontend/script.js` | Escape `photo.url` and `photo.fallback_url` in popup/gallery HTML, or build DOM nodes instead of template strings. | ⬜ |

## Medium Priority

| # | Issue | File / Area | Notes | Status |
|---|---|---|---|:---:|
| 4 | Duplicate processing setup logic | `src/processing.rs` | `process_photos_with_stats()` repeats folder-exists checks and `clear_database` handling. Remove duplicate block. | ⬜ |
| 5 | Database lock poisoning can panic app-wide | `src/database.rs` | Replace `RwLock::read/write().unwrap()` with error propagation or poison recovery. | ⬜ |
| 6 | Windows browser launch still shells through `cmd /C start` | `src/utils/browser.rs` | Current URL is local and low risk, but replace with safer direct platform launch for consistency with `reveal_file`. | ⬜ |

## Low Priority

| # | Issue | File / Area | Notes | Status |
|---|---|---|---|:---:|
| 7 | Inline `onclick` handlers and frontend globals | `frontend/index.html`, `frontend/script.js` | Move handlers to JS listeners and put app state/functions behind a small namespace or module pattern. | ⬜ |
| 8 | CDN JS/CSS no offline fallback | `frontend/index.html` | Bundle or vendor Leaflet, MarkerCluster, Heat, and PolylineDecorator assets for local-first/offline use. | ⬜ |
| 9 | No structured logging | Runtime logging | Current logging uses `println!` / `eprintln!`; consider a lightweight logging wrapper before adding heavier dependencies. | ⬜ |
| 10 | Dependency audit | `Cargo.toml` | Review outdated crates such as `rayon` and `kamadak-exif`; update only when binary size and platform compatibility stay acceptable. | ⬜ |

## Summary

Remaining: 2 high-priority, 3 medium-priority, 4 low-priority items.
