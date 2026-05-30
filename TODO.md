# PhotoMap TODO.md

## High Priority (Critical & Performance Bugs)

| # | Issue | File | Status |
|---|---|---|:---:|
| 1 | SSE events lost when no client connected — broadcast→mpsc→broadcast buffering | ✅ DONE | ✅ |
| 2 | O(n²) deduplication on batch insert — HashMap<String, PhotoMetadata> | ✅ DONE | ✅ |
| 3 | Full DB cloned on every image request — get_photo_by_relative_path() O(1) | ✅ DONE | ✅ |
| 4 | Server panic on HEIC path with non-UTF-8 chars — graceful error | ✅ DONE | ✅ |
| 5 | Geocoder init panics on corrupt/absent embedded asset — None fallback | ✅ DONE | ✅ |
| 6 | Settings toggle POST to wrong endpoint — POST → /api/update_settings | ✅ DONE | ✅ |
| 7 | CPU-bound image resizing on async worker thread — spawn_blocking | ✅ DONE | ✅ |
| 8 | std::sync::Mutex in async context — tokio::sync::Mutex for settings | ✅ DONE | ✅ |
| 9 | reprocess_photos / initiate_processing in tokio::spawn — moved to std::thread::spawn + blocking_send | ✅ DONE | ✅ |
| 10 | convert_heic_to_jpeg decode blocks async runtime — now via spawn_blocking boundary | ✅ DONE | ✅ |

## Medium Priority

| # | Issue | File | Status |
|---|---|---|:---:|
| 11 | bincode::deserialize_from untrusted cache — panic surface | ✅ DONE | ✅ |
| 12 | Command injection in reveal_file — raw JSON path into cmd /C start | ✅ DONE | ✅ |
| 13 | Duplicate HTML id exp-year-range-label — removed duplicate | ✅ DONE | ✅ |
| 14 | XSS via unsanitized filename in popup — escAttr/escHtml added | ✅ DONE | ✅ |
| 15 | Silent entries.flatten() skips permission errors | ✅ DONE | ✅ |
| 16 | Conflicting dual SSE heartbeats — kept axum KeepAlive only | ✅ DONE | ✅ |
| 17 | Stringly-typed sentinel error "GPS data not found" for control flow | ✅ DONE | ✅ |
| 18 | std::fs::read of large photos in async handler — tokio::fs::read | ✅ DONE | ✅ |
| 19 | Response::unwrap() on builder — propagated as Err | ✅ DONE | ✅ |
| 20 | Year range label: em-dash → hyphen, smaller font | ✅ DONE | ✅ |
| 21 | Marker URLs: Cyrillic paths broken — encodeURIComponent per segment | ✅ DONE | ✅ |

## Low Priority

| # | Issue | File | Status |
|---|---|---|:---:|
| 22 | No CLI args / hardcoded port 3001 | ✅ DONE | ✅ |
| 23 | CorsLayer::permissive() — restrict to localhost | ✅ DONE | ✅ |
| 24 | No structured logging (tracing) | ⬜ | |
| 25 | Outdated deps (rayon 1.8, kamadak-exif) | ⬜ | |
| 26 | CDN JS/CSS no offline fallback | ⬜ | |
| 27 | Polluting window global | ⬜ | |
| 28 | HEIC temp file no panic-cleanup | ✅ DONE | ✅ |

## Summary: 10 High-priority fixed, 0 remaining. Medium: 11 done / 0 pending. Low: 3 done / 4 pending.
