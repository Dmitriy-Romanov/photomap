# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PhotoMap Processor — это высокопроизводительное приложение на Rust для обработки фотографий, извлечения EXIF/GPS метаданных и отображения их на интерактивной карте. Использует in-memory базу данных с бинарным кэшем на диске для мгновенной загрузки больших коллекций.

## Common Development Commands

```bash
# Сборка релизной версии (оптимизированной)
cargo build --release

# Запуск приложения
./target/release/photomap_processor

# Запуск разработческой версии
cargo run

# Запуск тестов
cargo test

# Линтинг
cargo clippy

# Форматирование кода
cargo fmt

# Запуск с логированием debug уровня
RUST_LOG=debug cargo run
```

После запуска веб-интерфейс доступен по адресу http://127.0.0.1:3001

## Architecture

### Backend (Rust)

- **main.rs** — точка входа. Инициализирует логирование, базу данных, настройки, запускает HTTP-сервер. Обрабатывает загрузку кэша при старте.
- **server/** — Axum HTTP сервер с API для фронтенда
  - `mod.rs` — роутер и запуск сервера на порту 3001
  - `handlers.rs` — обработчики API (фото, изображения, настройки, обработка, shutdown)
  - `state.rs` — AppState с Arc<RwLock<>> для совместного использования
  - `events.rs` — SSE события для реального времени
- **database.rs** — in-memory база данных (Vec<PhotoMetadata>) с персистентностью через bincode в `photos.bin`
- **processing.rs** — сканирование папок и координация обработки фотографий
- **exif_parser/** — модуль для извлечения метаданных
  - `jpeg.rs` — EXIF из JPEG через kamadak-exif
  - `heic.rs` — EXIF из HEIC через libheif-rs
  - `gps_parser.rs` — low-level GPS парсер для повреждённых EXIF
  - `generic.rs` — общие функции для GPS координат и дат
- **image_processing.rs** — создание thumbnails, конвертация HEIC→JPEG, использует turbojpeg для скорости
- **geocoding.rs** — offline reverse geocoding через встроенную GeoNames базу (140k+ городов) с KD-Tree индексом
- **settings.rs** — управление настройками (INI файл), сохраняет up to 5 папок
- **folder_picker.rs** — нативные диалоги выбора папок (macOS/Windows/Linux)

### Frontend (embedded)

Фронтенд находится в `frontend/` и встраивается в бинарник через rust-embed.

- **index.html** — структура страницы с Leaflet картой
- **script.js** — логика карты, маркеров, кластеризации, галереи. Разделён на секции: API, DataService, MapController, UIController
- **style.css** — стили для карты, попапов, draggable panel

### Data Flow

1. При старте загружается кэш `photos.bin` если пути папок совпадают
2. Если кэш недействителен — сканируются папки, извлекается EXIF, metadata сохраняется в in-memory DB
3. Фронтенд запрашивает `/api/photos` — получает JSON с метаданными
4. Изображения генерируются on-demand при запросе `/api/marker/*`, `/api/thumbnail/*`, `/api/popup/*`
5. SSE `/api/events` используется для прогресса обработки

## Key Technical Details

- **Multi-folder support**: до 5 папок одновременно, хранятся в settings как массив
- **Lazy geocoding**: модуль geocoding инициализируется в фоне при старте
- **Image sizes** (constants.rs): MARKER=40px, THUMBNAIL=120px, GALLERY=240px, POPUP=1400px
- **Cross-platform**: Windows/macOS/Linux, используются разные native dialogs для каждой платформы
- **Single instance**: process_manager убивает существующие процессы перед запуском
- **Graceful shutdown**: `/api/shutdown` останавливает сервер корректно

## Configuration

Файл настроек автоматически создаётся в:
- macOS: `~/Library/Application Support/PhotoMap/config.ini`
- Windows: `%APPDATA%\PhotoMap\config.ini`
- Linux: `~/.config/PhotoMap/config.ini`

Содержит: папки, позицию панели, toggles (координаты, маршруты, heatmap, автозапуск браузера)
