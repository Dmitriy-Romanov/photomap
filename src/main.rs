use anyhow::{Context, Result};
use ignore::Walk;
use exif::{In, Reader, Tag, Value};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use image::GenericImageView;

// Structure to store metadata for each photo.
// `Serialize` is needed for JSON conversion.
#[derive(Serialize, Debug)]
struct ImageMetadata {
    filename: String,
    url: String,            // HTTP URL to original file
    fallback_url: String,   // HTTP URL to fallback JPEG for HEIC
    marker_icon: String,   // HTTP URL to 50px marker icon
    lat: f64,
    lng: f64,
    datetime: String,       // Date and time from EXIF (DD.MM.YYYY HH:MM)
}

// Processing statistics
#[derive(Debug)]
struct ProcessingStats {
    total_files: usize,
    processed_photos: usize,
    no_gps_files: usize,
    heic_files: usize,
    jpeg_files: usize,
    other_files: usize,
    processing_time_secs: f64,
    avg_time_per_file_ms: f64,
}

const THUMBNAIL_DIR: &str = ".thumbnails";
const MARKER_SIZE: u32 = 50;
const OUTPUT_FILE: &str = "geodata.js";
const MAP_HTML_FILE: &str = "map.html";

/// Очищает директорию с миниатюрами для чистоты эксперимента
fn clean_thumbnails_directory(thumbnails_path: &str) -> Result<()> {
    println!("🧹 Очистка директории миниатюр: {}", thumbnails_path);

    let path = Path::new(thumbnails_path);
    if path.exists() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() {
                if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
                    // Удаляем только файлы миниатюр, не трогая другие файлы
                    if filename.ends_with(".png") || filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
                        match fs::remove_file(&file_path) {
                            Ok(_) => println!("  🗑️  Удален: {}", filename),
                            Err(e) => eprintln!("  ⚠️  Не удалось удалить {}: {}", filename, e),
                        }
                    }
                }
            }
        }
        println!("✅ Очистка завершена");
    }

    Ok(())
}

// Встроенный HTML для карты
const MAP_HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>PhotoMap</title>
    <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
    <link rel="stylesheet" href="https://unpkg.com/leaflet.markercluster@1.5.3/dist/MarkerCluster.css" />
    <link rel="stylesheet" href="https://unpkg.com/leaflet.markercluster@1.5.3/dist/MarkerCluster.Default.css" />
    <style>
        body { margin: 0; padding: 0; }
        #map { height: 100vh; width: 100vw; }
        .popup-image {
            max-width: 700px;
            max-height: 700px;
            width: auto;
            height: auto;
            display: block;
            margin: 0 auto;
            object-fit: contain;
        }
        .leaflet-popup-content {
            width: auto !important;
            min-width: 300px !important;
            max-width: 720px !important;
            padding: 12px !important;
            margin: 0 !important;
            text-align: center;
        }
        .leaflet-popup-content p {
            margin: 8px 0 0 0;
            padding: 0;
            text-align: left;
        }
        .popup-date {
            font-size: 0.9em;
            color: #666;
            margin-top: 8px;
        }
        .popup-filename {
            margin-bottom: 8px;
        }
        .popup-image-container {
            text-align: center;
            margin: 0 auto;
        }
    </style>
</head>
<body>

    <div id="map"></div>

    <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
    <script src="https://unpkg.com/leaflet.markercluster@1.5.3/dist/leaflet.markercluster.js"></script>
    
    <!-- Загружаем данные как JS-файл, чтобы обойти CORS -->
    <script src="geodata.js"></script>

    <script>
        // Инициализация карты
        const map = L.map('map').setView([0, 0], 2);
        
        // Добавляем слой тайлов OpenStreetMap
        L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
            maxZoom: 19,
            attribution: '&copy; <a href="http://www.openstreetmap.org/copyright">OpenStreetMap</a>'
        }).addTo(map);

        // Создаем группу для кластеризации маркеров
        const markerClusterGroup = L.markerClusterGroup({
            chunkedLoading: true // Оптимизация для большого количества маркеров
        });

        // Проверяем, загрузились ли данные
        if (typeof photoData !== 'undefined' && photoData.length > 0) {
            const bounds = L.latLngBounds();

            // Детекция поддержки HEIC в браузере
            function supportsHEIC() {
                const canvas = document.createElement('canvas');
                const ctx = canvas.getContext('2d');
                // Проверяем поддержку через canvas
                if (!ctx) return false;

                // Проверяем MIME type
                const heicMimeTypes = ['image/heic', 'image/heif', 'image/heic-sequence', 'image/heif-sequence'];
                return heicMimeTypes.some(mimeType => ctx.drawImage &&
                    new Image().onload &&
                    new Image().onerror === null);
            }

            const heicSupported = supportsHEIC();
            console.log('HEIC support detected:', heicSupported);

            photoData.forEach(function(photo) {
                // Создаем иконку маркера из маленькой иконки
                const customIcon = L.icon({
                    iconUrl: photo.marker_icon,
                    iconSize: [50, 50],
                    iconAnchor: [25, 25],
                    popupAnchor: [0, -25],
                    className: 'custom-marker' // для кастомизации через CSS
                });

                // Создаем маркер
                const marker = L.marker([photo.lat, photo.lng], { icon: customIcon });

                // Создаем содержимое для всплывающего окна (popup)
                const isHeic = photo.filename.toLowerCase().endsWith('.heic');

                // Для HEIC файлов в браузерах без поддержки - используем ленивую конвертацию
                if (isHeic && !heicSupported) {
                    const popupContent = `
                        <div id="popup-${photo.filename.replace(/[^a-zA-Z0-9]/g, '_')}">
                            <p class="popup-filename"><strong>${photo.filename}</strong></p>
                            <p class="popup-date">${photo.datetime}</p>
                            <p style="font-size: 0.8em; color: #666;">Загрузка HEIC изображения...</p>
                            <img src="${photo.marker_icon}" alt="${photo.filename}" style="width: 50px; height: 50px; opacity: 0.3;">
                        </div>
                    `;

                    marker.bindPopup(popupContent);

                    // При открытии popup начинаем конвертацию
                    marker.on('popupopen', function() {
                        convertHeicToJpeg(photo);
                    });
                } else {
                    // Для обычных изображений или HEIC в поддерживающих браузерах
                    const imageUrl = isHeic && !heicSupported ? photo.fallback_url : photo.url;

                    const popupContent = `
                        <div class="popup-image-container">
                            <img src="${imageUrl}" alt="${photo.filename}" class="popup-image">
                        </div>
                        <p class="popup-date">${photo.datetime}</p>
                        <p class="popup-filename"><strong>${photo.filename}</strong></p>
                        ${isHeic && !heicSupported ? '<p style="font-size: 0.8em; color: #666;">HEIC → JPEG (браузер не поддерживает)</p>' : ''}
                    `;
                    marker.bindPopup(popupContent);
                }

                // Добавляем маркер в группу кластеров
                markerClusterGroup.addLayer(marker);

                // Расширяем границы карты, чтобы все маркеры были видны
                bounds.extend([photo.lat, photo.lng]);
            });

            // Добавляем группу маркеров на карту
            map.addLayer(markerClusterGroup);

            // Масштабируем карту так, чтобы были видны все маркеры
            map.fitBounds(bounds);

        } else {
            // Если данных нет, показываем сообщение
            L.popup()
             .setLatLng(map.getCenter())
             .setContent('Фотографии с GPS-данными не найдены. Запустите photomap_processor для их создания.')
             .openOn(map);
        }

        // Функция для ленивой конвертации HEIC в JPEG с улучшенным UX
        async function convertHeicToJpeg(photo) {
            const popupId = `popup-${photo.filename.replace(/[^a-zA-Z0-9]/g, '_')}`;
            const popupElement = document.getElementById(popupId);

            if (!popupElement) return;

            let startTime = Date.now();
            let dots = 0;

            // Функция обновления анимации загрузки
            const updateLoadingAnimation = () => {
                dots = (dots + 1) % 4;
                const dotsText = '.'.repeat(dots) + ' '.repeat(3 - dots);
                popupElement.querySelector('.loading-text').textContent =
                    `Конвертация HEIC → JPEG${dotsText}`;
            };

            try {
                // Показываем улучшенный статус загрузки
                popupElement.innerHTML = `
                    <div class="heic-conversion-popup">
                        <p class="popup-filename"><strong>${photo.filename}</strong></p>
                        <p class="popup-date">${photo.datetime}</p>
                        <div class="loading-container">
                            <p class="loading-text" style="font-size: 0.9em; color: #666; margin: 10px 0;">Конвертация HEIC → JPEG...</p>
                            <div class="loading-bar" style="width: 100%; height: 6px; background-color: #f0f0f0; border-radius: 3px; overflow: hidden; margin: 10px 0;">
                                <div class="loading-progress" style="height: 100%; background: linear-gradient(90deg, #3498db, #2ecc71); width: 0%; border-radius: 3px; transition: width 0.3s ease;"></div>
                            </div>
                            <div class="loading-spinner" style="text-align: center; padding: 10px;">
                                <div style="border: 3px solid #f3f3f3; border-top: 3px solid #3498db; border-radius: 50%; width: 30px; height: 30px; animation: spin 1s linear infinite; display: inline-block;"></div>
                            </div>
                            <p class="loading-info" style="font-size: 0.75em; color: #888; text-align: center;">Обработка изображения Apple HEIC...</p>
                        </div>
                    </div>
                    <style>
                        @keyframes spin {
                            0% { transform: rotate(0deg); }
                            100% { transform: rotate(360deg); }
                        }
                        .heic-conversion-popup {
                            min-width: 250px;
                            text-align: center;
                        }
                    </style>
                `;

                // Запускаем анимацию загрузки
                const loadingInterval = setInterval(() => {
                    updateLoadingAnimation();

                    // Имитация прогресса
                    const elapsed = Date.now() - startTime;
                    const progress = Math.min(90, (elapsed / 3000) * 100); // 90% за 3 секунды
                    const progressBar = popupElement.querySelector('.loading-progress');
                    if (progressBar) {
                        progressBar.style.width = progress + '%';
                    }

                    // Обновляем информационный текст в зависимости от времени
                    const infoElement = popupElement.querySelector('.loading-info');
                    if (infoElement) {
                        if (elapsed < 1000) {
                            infoElement.textContent = 'Инициализация конвертации...';
                        } else if (elapsed < 2000) {
                            infoElement.textContent = 'Чтение HEIC данных...';
                        } else if (elapsed < 3000) {
                            infoElement.textContent = 'Создание JPEG версии...';
                        } else {
                            infoElement.textContent = 'Финализация...';
                        }
                    }
                }, 200);

                // Вызываем API для конвертации
                const response = await fetch(photo.fallback_url);

                // Останавливаем анимацию
                clearInterval(loadingInterval);

                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                }

                // Получаем размер файла для информативности
                const contentLength = response.headers.get('content-length');
                const fileSize = contentLength ? (contentLength / 1024 / 1024).toFixed(1) : 'неизвестный';
                const conversionTime = ((Date.now() - startTime) / 1000).toFixed(1);

                // Получаем URL для сконвертированного изображения
                const imageUrl = photo.fallback_url;

                // Обновляем popup с загруженным изображением и статистикой
                popupElement.innerHTML = `
                    <div class="heic-success-popup">
                        <div class="popup-image-container">
                            <img src="${imageUrl}" alt="${photo.filename}" class="popup-image" style="opacity: 0; transition: opacity 0.5s ease-in-out;">
                        </div>
                        <p class="popup-date">${photo.datetime}</p>
                        <p class="popup-filename"><strong>${photo.filename}</strong></p>
                        <div class="conversion-stats" style="background: #e8f5e8; padding: 8px; border-radius: 4px; margin: 8px 0; font-size: 0.8em; color: #2e7d32;">
                            <div style="display: flex; justify-content: space-between; margin: 2px 0;">
                                <span>⚡ Конвертировано за:</span>
                                <strong>${conversionTime} сек</strong>
                            </div>
                            <div style="display: flex; justify-content: space-between; margin: 2px 0;">
                                <span>📏 Размер файла:</span>
                                <strong>${fileSize} MB</strong>
                            </div>
                            <div style="text-align: center; margin-top: 4px; font-weight: bold; color: #1b5e20;">
                                ✅ HEIC → JPEG (по запросу)
                            </div>
                        </div>
                    </div>
                `;

                // Анимируем появление изображения
                setTimeout(() => {
                    const img = popupElement.querySelector('.popup-image');
                    if (img) img.style.opacity = '1';
                }, 100);

                console.log(`✅ HEIC успешно сконвертирован: ${photo.filename} (${conversionTime}s, ${fileSize}MB)`);

            } catch (error) {
                clearInterval(loadingInterval); // Останавливаем анимацию при ошибке

                console.error('❌ Ошибка конвертации HEIC:', error);
                popupElement.innerHTML = `
                    <div class="heic-error-popup">
                        <p class="popup-filename"><strong>${photo.filename}</strong></p>
                        <p class="popup-date">${photo.datetime}</p>
                        <div class="error-container" style="background: #ffebee; padding: 12px; border-radius: 4px; margin: 8px 0; border-left: 4px solid #f44336;">
                            <div style="color: #c62828; font-weight: bold; margin-bottom: 8px;">
                                ❌ Ошибка конвертации HEIC
                            </div>
                            <div style="color: #666; font-size: 0.85em; margin-bottom: 8px;">
                                Код ошибки: ${error.message}
                            </div>
                            <div style="color: #888; font-size: 0.75em;">
                                Возможные причины:<br>
                                • Файл поврежден или не является HEIC<br>
                                • Проблемы с доступом к ImageMagick<br>
                                • Недостаточно места на диске
                            </div>
                        </div>
                        <div style="text-align: center; margin-top: 8px;">
                            <button onclick="location.reload()" style="background: #2196F3; color: white; border: none; padding: 6px 12px; border-radius: 3px; cursor: pointer; font-size: 0.8em;">
                                🔄 Перезагрузить страницу
                            </button>
                        </div>
                    </div>
                `;
            }
        }
    </script>

</body>
</html>"#;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🗺️  PhotoMap Processor starting...");

    // 0. Create map.html if it doesn't exist
    if !std::path::Path::new(MAP_HTML_FILE).exists() {
        println!("📄 Creating map.html...");
        create_map_html()?;
        println!("✅ map.html created in current directory: {}", MAP_HTML_FILE);
    } else {
        println!("📄 map.html already exists in current directory: {}", MAP_HTML_FILE);
    }

    // 1. Create thumbnails directory and clean it for clean experiment
    fs::create_dir_all(THUMBNAIL_DIR)
        .with_context(|| format!("Failed to create thumbnails directory: {}", THUMBNAIL_DIR))?;

    // Clean thumbnails directory for clean experiment
    clean_thumbnails_directory(THUMBNAIL_DIR)?;

    // 2. Get list of all files in photos directory
    println!("🔍 Scanning photos directory...");
    let photos_dir = Path::new("/Users/dmitriiromanov/claude/photomap/photos");
    if !photos_dir.exists() {
        println!("❌ Photos directory not found: {}", photos_dir.display());
        return Ok(());
    }
    println!("📂 Photos directory: {}", photos_dir.display());

    // Create walker for photos directory only
    let walker = Walk::new(photos_dir);
    let files: Vec<PathBuf> = walker
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|e| {
            // Check that file is in photos directory
            e.path().starts_with(&photos_dir)
        })
        .filter(|e| {
            // Exclude system directories and hidden files
            let path = e.path();
            if let Some(components) = path.components().collect::<Vec<_>>().get(1..) {
                for component in components {
                    if let Some(name) = component.as_os_str().to_str() {
                        if name.starts_with('.') || name == "node_modules" || name == "target" || name == ".git" {
                            return false;
                        }
                    }
                }
            }
            true
        })
        .filter(|e| e.file_type().map_or(false, |ft| ft.is_file()))
        .map(|e| e.into_path())
        .collect();
    println!("✅ Found {} files in photos directory. Starting processing...", files.len());

    // 3. Process files in parallel using Rayon with timing
    let start_time = std::time::Instant::now();
    let total_files = files.len();

    println!("📊 Processing {} files with parallel optimization...", total_files);

    let photo_data: Vec<ImageMetadata> = files
        .par_iter() // <-- Parallelism magic!
        .filter_map(|path| process_file(path).ok()) // Filter out files that couldn't be processed
        .collect();

    let processing_time = start_time.elapsed();
    let processing_secs = processing_time.as_secs_f64();
    let avg_time_per_file_ms = if total_files > 0 {
        (processing_secs * 1000.0) / total_files as f64
    } else {
        0.0
    };

    // Calculate statistics
    let stats = ProcessingStats {
        total_files: files.len(),
        processed_photos: photo_data.len(),
        no_gps_files: files.len() - photo_data.len(),
        heic_files: photo_data.iter().filter(|p| p.filename.ends_with(".HEIC")).count(),
        jpeg_files: photo_data.iter().filter(|p| p.filename.to_lowercase().ends_with(".jpg")).count(),
        other_files: photo_data.iter().filter(|p| !p.filename.to_lowercase().ends_with(".jpg") && !p.filename.ends_with(".HEIC")).count(),
        processing_time_secs: processing_secs,
        avg_time_per_file_ms,
    };

    // Print processing statistics with performance metrics
    println!("\n📊 Статистика обработки:");
    println!("   🔍 Всего файлов проверено: {}", stats.total_files);
    println!("   📸 Обработано фотографий: {}", stats.processed_photos);
    println!("   🗺️  С GPS-данными: {}", stats.processed_photos);
    println!("   ❌ Без GPS: {}", stats.no_gps_files);
    println!("   📱 HEIC файлов: {}", stats.heic_files);
    println!("   📷 JPEG файлов: {}", stats.jpeg_files);
    if stats.other_files > 0 {
        println!("   📄 Других форматов: {}", stats.other_files);
    }
    println!("   ⏱️  Время обработки: {:.2} сек", stats.processing_time_secs);
    println!("   📈 Среднее время на файл: {:.1} мс", stats.avg_time_per_file_ms);

    // Performance prediction for large collections
    if stats.total_files >= 100 {
        let predicted_10k_time = (stats.avg_time_per_file_ms * 10000.0) / 1000.0;
        let predicted_100k_time = (stats.avg_time_per_file_ms * 100000.0) / 1000.0;

        println!("\n🔮 Прогноз производительности:");
        println!("   📊 Для 10,000 фото: ~{:.1} минут", predicted_10k_time / 60.0);
        println!("   📊 Для 100,000 фото: ~{:.1} минут", predicted_100k_time / 60.0);

        if stats.heic_files > 0 {
            println!("   💡 Экономия от ленивой конвертации HEIC: ~{}%",
                ((stats.heic_files as f64 / stats.processed_photos as f64) * 95.0).round());
        }
    }

    // 4. Write result to geodata.js
    write_geodata_js(&photo_data)?;

    println!(
        "\n🎉 Обработка завершена! Данные сохранены в '{}'.",
        OUTPUT_FILE
    );

    // Start HTTP server
    start_http_server(stats.processed_photos).await
}

/// Обрабатывает один файл: извлекает EXIF, GPS, создает миниатюру.
fn process_file(path: &Path) -> Result<ImageMetadata> {
    // Проверяем расширение файла
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase());
    
    // Базовый список поддерживаемых форматов (HEIC теперь всегда поддерживается)
    let supported_formats = ["jpg", "jpeg", "png", "tiff", "tif", "webp", "bmp", "gif", "heic", "heif", "avif"];

    if !supported_formats.contains(&ext.as_deref().unwrap_or("")) {
        anyhow::bail!("Файл не является поддерживаемым изображением (поддерживается: JPG, PNG, WebP, TIFF, BMP, GIF, HEIC, HEIF, AVIF)");
    }

    // Проверяем, это HEIC или нет (теперь всегда поддерживается)
    let is_heif = matches!(ext.as_deref(), Some("heic") | Some("heif") | Some("avif"));

    // --- Извлечение GPS и даты ---
    let (lat, lng, datetime) = if is_heif {
        // Пытаемся извлечь метаданные из HEIC с помощью нашего парсера
        match extract_metadata_from_heif_custom(path) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("⚠️  Ошибка при обработке HEIC файла {}: {}", path.display(), e);
                anyhow::bail!("Не удалось обработать HEIC файл")
            }
        }
    } else {
        // Для стандартных форматов используем наши парсеры
        let ext = path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        if ext == "jpg" || ext == "jpeg" {
            // Используем наш собственный JPEG парсер
            match extract_metadata_from_jpeg_custom(path) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("⚠️  Ошибка при обработке JPEG файла {}: {}", path.display(), e);
                    anyhow::bail!("Не удалось обработать JPEG файл")
                }
            }
        } else {
            // Для остальных форматов (PNG, TIFF и т.д.) оставляем старый метод
            let file = fs::File::open(path)?;
            let mut bufreader = std::io::BufReader::new(&file);
            let exifreader = Reader::new();
            let exif = exifreader.read_from_container(&mut bufreader)?;

            let lat = get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?;
            let lng = get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?;

            if lat.is_none() || lng.is_none() {
                anyhow::bail!("GPS-данные не найдены");
            }

            let datetime = get_datetime_from_exif(&exif).unwrap_or_else(|| "Дата неизвестна".to_string());

            (lat.unwrap(), lng.unwrap(), datetime)
        }
    };

    // --- Создание миниатюры ---
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::Error::msg("Некорректное имя файла"))?;

    let icon_path = generate_marker_icon_path(path)?;

    // --- Обработка HEIC файлов ---
    if is_heif {
        // НЕ создаем JPEG версию заранее - только если понадобится для иконки
        let icon_result = create_heic_thumbnail(path, &icon_path)?;

        match icon_result {
            Some(_) => {
                // Иконка создана успешно через ImageMagick/sips
            }
            None => {
                // Создаем информационную заглушку для иконки
                create_info_thumbnail(path, &icon_path)?;
            }
        }

        // --- Формирование результата для HEIC (с ленивой конвертацией) ---
        let metadata = ImageMetadata {
            filename: filename.to_string(),
            url: format!("/photos/{}", filename), // Нативный HEIC
            fallback_url: format!("/convert-heic?filename={}", filename), // API endpoint для конвертации по запросу
            marker_icon: format!("/.thumbnails/{}.png", filename.trim_end_matches(".HEIC").trim_end_matches(".heic").trim_end_matches(".jpg").trim_end_matches(".jpeg")), // PNG иконка
            lat,
            lng,
            datetime,
        };
        return Ok(metadata);
    } else {
        // Обработка обычных файлов (JPEG, PNG и т.д.)
        let png_icon_path = icon_path.with_extension("png");
        create_marker_icon(path, &png_icon_path)?;

        // --- Формирование результата для обычных файлов ---
        let metadata = ImageMetadata {
            filename: filename.to_string(),
            url: format!("/photos/{}", filename), // Оригинал
            fallback_url: format!("/photos/{}", filename), // Такой же fallback для обычных файлов
            marker_icon: format!("/.thumbnails/{}.png", filename.trim_end_matches(".HEIC").trim_end_matches(".heic").trim_end_matches(".jpg").trim_end_matches(".jpeg")), // PNG иконка
            lat,
            lng,
            datetime,
        };
        return Ok(metadata);
    }

    // Этот код недостижим, но нужен для компиляции
    unreachable!("Unreachable code")
}

/// Вспомогательная функция для преобразования GPS-координат из EXIF в f64.
fn get_gps_coord(
    exif: &exif::Exif,
    coord_tag: Tag,
    ref_tag: Tag,
) -> Result<Option<f64>> {
    let coord_field = exif.get_field(coord_tag, In::PRIMARY);
    let ref_field = exif.get_field(ref_tag, In::PRIMARY);

    if let (Some(coord), Some(ref_val)) = (coord_field, ref_field) {
        if let Value::Rational(ref vec) = coord.value {
            if vec.len() == 3 {
                let d = vec[0].to_f64();
                let m = vec[1].to_f64();
                let s = vec[2].to_f64();
                let mut decimal = d + (m / 60.0) + (s / 3600.0);

                // Применяем референс (S/W - отрицательные значения)
                if let Some(ref_val) = ref_val.display_value().to_string().chars().next() {
                    if ref_val == 'S' || ref_val == 'W' {
                        decimal *= -1.0;
                    }
                }
                return Ok(Some(decimal));
            }
        }
    }
    Ok(None)
}

/// Применяет EXIF-ориентацию к изображению на основе тега Orientation.
/// EXIF-тег Orientation (0x0112) определяет, как нужно повернуть изображение:
/// 1=нормально, 2=отразить горизонтально, 3=повернуть на 180°, 
/// 4=отразить вертикально, 5=повернуть на 90° влево и отразить,
/// 6=повернуть на 90° вправо, 7=повернуть на 90° вправо и отразить,
/// 8=повернуть на 90° влево
fn apply_exif_orientation(source_path: &Path, img: image::DynamicImage) -> Result<image::DynamicImage> {
    let file = match fs::File::open(source_path) {
        Ok(f) => f,
        Err(_) => return Ok(img), // Если не удалось открыть - возвращаем изображение как есть
    };
    
    let mut bufreader = std::io::BufReader::new(&file);
    let exifreader = Reader::new();
    
    // Пытаемся прочитать EXIF, но если не получилось - просто возвращаем оригинальное изображение
    let exif = match exifreader.read_from_container(&mut bufreader) {
        Ok(e) => e,
        Err(_) => return Ok(img),
    };
    
    // Ищем тег ориентации (0x0112)
    let orientation = exif
        .get_field(Tag::Orientation, In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(1); // По умолчанию 1 (нормальная ориентация)
    
    // Применяем трансформацию в зависимости от значения ориентации
    let rotated = match orientation {
        1 => img, // Нормально
        2 => img.fliph(), // Отразить горизонтально
        3 => img.rotate180(), // Повернуть на 180°
        4 => img.flipv(), // Отразить вертикально
        5 => img.rotate270().fliph(), // Повернуть на 270° (90° влево) и отразить
        6 => img.rotate90(), // Повернуть на 90° вправо
        7 => img.rotate90().fliph(), // Повернуть на 90° и отразить
        8 => img.rotate270(), // Повернуть на 270° (90° влево)
        _ => img, // Неизвестное значение - оставляем как есть
    };
    
    Ok(rotated)
}

/// Создает маленькую иконку маркера для изображения (50x50px PNG с прозрачностью и центрированием).
fn create_marker_icon(source_path: &Path, icon_path: &Path) -> Result<()> {
    let mut img = image::open(source_path)
        .with_context(|| format!("Не удалось открыть изображение: {:?}", source_path))?;

    // Применяем EXIF-ориентацию
    img = apply_exif_orientation(source_path, img)?;

    // Создаем квадратное изображение 50x50 с ПРОЗРАЧНЫМ фоном
    let mut canvas = image::RgbaImage::from_fn(MARKER_SIZE, MARKER_SIZE, |_, _| {
        image::Rgba([0, 0, 0, 0]) // Полностью прозрачный фон
    });

    // Масштабируем изображение с сохранением пропорций
    let scaled = img.resize(MARKER_SIZE, MARKER_SIZE, image::imageops::FilterType::Lanczos3);

    // Получаем размеры и вычисляем позицию для центрирования
    let (width, height) = scaled.dimensions();
    let x_offset = (MARKER_SIZE - width as u32) / 2;
    let y_offset = (MARKER_SIZE - height as u32) / 2;

    // Копируем масштабированное изображение в центр
    image::imageops::overlay(&mut canvas, &scaled.to_rgba8(), x_offset as i64, y_offset as i64);

    // Сохраняем результат как PNG
    let final_img = image::DynamicImage::ImageRgba8(canvas);
    final_img.save_with_format(icon_path, image::ImageFormat::Png)?;
    Ok(())
}

/// Создает иконку маркера из уже декодированного image::DynamicImage (для HEIC/AVIF).
#[allow(dead_code)]
fn create_marker_icon_from_dynamic_image(img: &image::DynamicImage, icon_path: &Path) -> Result<()> {
    // Создаем квадратное изображение 50x50 с ПРОЗРАЧНЫМ фоном и центрированием
    let mut canvas = image::RgbaImage::from_fn(MARKER_SIZE, MARKER_SIZE, |_, _| {
        image::Rgba([0, 0, 0, 0]) // Полностью прозрачный фон
    });

    // Масштабируем изображение с сохранением пропорций
    let scaled = img.resize(MARKER_SIZE, MARKER_SIZE, image::imageops::FilterType::Lanczos3);

    // Получаем размеры и вычисляем позицию для центрирования
    let (width, height) = scaled.dimensions();
    let x_offset = (MARKER_SIZE - width as u32) / 2;
    let y_offset = (MARKER_SIZE - height as u32) / 2;

    // Копируем масштабированное изображение в центр
    image::imageops::overlay(&mut canvas, &scaled.to_rgba8(), x_offset as i64, y_offset as i64);

    // Сохраняем результат как PNG
    let final_img = image::DynamicImage::ImageRgba8(canvas);
    final_img.save_with_format(icon_path, image::ImageFormat::Png)?;
    Ok(())
}

/// Генерирует уникальный и безопасный путь для иконки маркера.
fn generate_marker_icon_path(original_path: &Path) -> Result<PathBuf> {
    let filename = original_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::Error::msg("Некорректное имя файла"))?;
    let safe_filename = filename.replace('/', "_").replace('\\', "_");
    Ok(Path::new(THUMBNAIL_DIR).join(safe_filename))
}

/// Генерирует путь для JPEG версии HEIC файла для popup.
fn generate_heic_jpeg_path(heic_path: &Path) -> Result<PathBuf> {
    let filename = heic_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::Error::msg("Некорректное имя файла"))?;
    let jpeg_filename = filename.replace(".HEIC", "_popup.jpg").replace(".heic", "_popup.jpg");
    Ok(Path::new(THUMBNAIL_DIR).join(jpeg_filename))
}

/// Создает JPEG версию HEIC файла для popup (полное качество).
fn create_heic_jpeg_for_popup(heic_path: &Path, jpeg_path: &Path) -> Result<bool> {
    // Пытаемся использовать ImageMagick
    if let Ok(output) = std::process::Command::new("magick")
        .arg(heic_path)
        .arg("-quality")
        .arg("90")
        .arg(jpeg_path)
        .output()
    {
        if output.status.success() {
            eprintln!("✅ Создана JPEG версия HEIC для popup: {}", heic_path.display());
            return Ok(true);
        }
    }

    // Пытаемся использовать sips (только на macOS)
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sips")
            .arg("-s")
            .arg("format")
            .arg("jpeg")
            .arg("-s")
            .arg("formatOptions")
            .arg("90")
            .arg(heic_path)
            .arg("--out")
            .arg(jpeg_path)
            .output()
        {
            if output.status.success() {
                eprintln!("✅ Создана JPEG версия HEIC через sips: {}", heic_path.display());
                return Ok(true);
            }
        }
    }

    Ok(false) // Не удалось создать JPEG
}

/// Записывает данные в файл geodata.js в формате JavaScript-переменной.
fn write_geodata_js(data: &[ImageMetadata]) -> Result<()> {
    let file = fs::File::create(OUTPUT_FILE)?;
    let mut writer = BufWriter::new(file);

    // Записываем префикс JS-переменной
    writeln!(writer, "var photoData = ")?;

    // Используем to_writer_pretty для потоковой записи без загрузки всего JSON в память
    serde_json::to_writer_pretty(&mut writer, data)?;

    // Записываем суффикс
    writeln!(writer, ";")?;

    Ok(())
}

/// Создает файл map.html с встроенным HTML кодом.
fn create_map_html() -> Result<()> {
    fs::write(MAP_HTML_FILE, MAP_HTML_TEMPLATE)
        .with_context(|| format!("Не удалось создать файл: {}", MAP_HTML_FILE))?;
    Ok(())
}

/// Паузирует программу и ждет ввода пользователя перед закрытием.
fn pause_and_wait_for_input() -> Result<()> {
    use std::io::Read;
    
    println!("\n✋ Нажмите любую клавишу для выхода...");
    let _ = std::io::stdin().read(&mut [0u8; 1]);
    
    Ok(())
}

/// Извлекает дату и время съемки из EXIF-данных.
fn get_datetime_from_exif(exif: &exif::Exif) -> Option<String> {
    // Сначала пробуем стандартный тег DateTimeOriginal (если он есть),
    // затем пробуем более общий тег DateTime.
    let try_tags = [Tag::DateTimeOriginal, Tag::DateTime];

    for &tag in &try_tags {
        if let Some(field) = exif.get_field(tag, In::PRIMARY) {
            if let exif::Value::Ascii(ref vec) = field.value {
                if let Some(datetime_str) = vec.first() {
                    // Формат EXIF обычно: "YYYY:MM:DD HH:MM:SS"
                    if let Ok(s) = std::str::from_utf8(datetime_str) {
                        let parts: Vec<&str> = s.split(' ').collect();
                        if parts.len() == 2 {
                            let date_parts: Vec<&str> = parts[0].split(':').collect();
                            let time_parts: Vec<&str> = parts[1].split(':').collect();

                            if date_parts.len() == 3 && time_parts.len() >= 2 {
                                let year = date_parts[0];
                                let month = date_parts[1];
                                let day = date_parts[2];
                                let hour = time_parts[0];
                                let min = time_parts[1];

                                return Some(format!("Дата съемки: {}.{}.{} {}:{}", day, month, year, hour, min));
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

// ============================================================
// ============================================================
// HEIC/AVIF Support (built-in native parsers)
// ============================================================

// Native HEIC parser without external libraries
fn extract_metadata_from_heif_custom(path: &Path) -> Result<(f64, f64, String)> {
    let data = std::fs::read(path)?;

    // Ищем начало EXIF данных в HEIC файле
    // EXIF обычно хранится после "Exif" маркера
    let mut exif_start = None;

    // Ищем последовательность байт "Exif" в файле
    for i in 0..data.len().saturating_sub(4) {
        if data[i] == b'E' && data[i+1] == b'x' && data[i+2] == b'i' && data[i+3] == b'f' {
            // Пропускаем "Exif" и 2 байта после него
            exif_start = Some(i + 6);
            break;
        }
    }

    if let Some(start) = exif_start {
        // Ищем начало TIFF данных (II или MM)
        let mut tiff_start = start;
        while tiff_start < data.len().saturating_sub(1) {
            if (data[tiff_start] == b'I' && data[tiff_start + 1] == b'I') ||
               (data[tiff_start] == b'M' && data[tiff_start + 1] == b'M') {
                break;
            }
            tiff_start += 1;
        }

        if tiff_start < data.len().saturating_sub(1) {
            // Используем стандартную библиотеку exif для парсинга найденных данных
            if let Ok(exif) = exif::Reader::new().read_raw(data[tiff_start..].to_vec()) {
                let lat = get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?;
                let lng = get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?;
                let datetime = get_datetime_from_exif(&exif).unwrap_or_else(|| "Дата неизвестна".to_string());

                if lat.is_some() && lng.is_some() {
                    return Ok((lat.unwrap(), lng.unwrap(), datetime));
                }
            }
        }
    }

    anyhow::bail!("GPS-данные не найдены в HEIF файле")
}

// Собственный парсер JPEG без сторонних библиотек
fn extract_metadata_from_jpeg_custom(path: &Path) -> Result<(f64, f64, String)> {
    let data = std::fs::read(path)?;

    // Ищем EXIF сегмент в JPEG файле
    // EXIF хранится в APP1 сегменте (FF E1)
    let mut i = 0;
    let mut found_exif_segment = false;

    while i < data.len().saturating_sub(4) {
        if data[i] == 0xFF && data[i+1] == 0xE1 {
            // Нашли APP1 сегмент, читаем его длину
            if i + 4 < data.len() {
                let segment_length = ((data[i+2] as u16) << 8) | (data[i+3] as u16);

                // Проверяем, что это EXIF сегмент
                if i + 8 < data.len() &&
                   data[i+4] == b'E' && data[i+5] == b'x' &&
                   data[i+6] == b'i' && data[i+7] == b'f' {

                    found_exif_segment = true;
                    // EXIF данные начинаются после 6 байт (FF E1 + 2 байта длины + 4 байта "Exif")
                    let mut exif_start = i + 8;
                    let exif_end = i + segment_length as usize;

                    // Пропускаем возможные нулевые байты перед TIFF заголовком
                    while exif_start < exif_end && data[exif_start] == 0 {
                        exif_start += 1;
                    }

                    if exif_end <= data.len() && exif_start + 2 < data.len() {
                        // Проверяем наличие TIFF заголовка
                        if (data[exif_start] == b'I' && data[exif_start + 1] == b'I') ||
                           (data[exif_start] == b'M' && data[exif_start + 1] == b'M') {

                            // Используем стандартную библиотеку exif для парсинга
                            if let Ok(exif) = exif::Reader::new().read_raw(data[exif_start..exif_end].to_vec()) {
                                let lat = get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?;
                                let lng = get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?;
                                let datetime = get_datetime_from_exif(&exif).unwrap_or_else(|| "Дата неизвестна".to_string());

                                if lat.is_some() && lng.is_some() {
                                    return Ok((lat.unwrap(), lng.unwrap(), datetime));
                                }
                            }
                        }
                    }
                }
            }
        }
        i += 1;
    }

    
    anyhow::bail!("GPS-данные не найдены в JPEG файле")
}

/// Создает миниатюру для HEIC файла с использованием системных утилит
/// Возвращает Some(PathBuf) с путем к созданной миниатюре или None если не удалось
fn create_heic_thumbnail(heic_path: &Path, _thumbnail_path: &Path) -> Result<Option<PathBuf>> {
    // Пытаемся использовать ImageMagick (magick) если доступен
    // Создаем JPEG миниатюру для HEIC файла (временный файл)
    let jpeg_thumbnail_path = _thumbnail_path.with_extension("temp.jpg");

        if let Ok(output) = std::process::Command::new("magick")
        .arg(heic_path)
        .arg("-resize")
        .arg(&format!("{}x{}", MARKER_SIZE * 4, MARKER_SIZE * 4)) // Создаем большой квадрат 200x200
        .arg("-quality")
        .arg("80")
        .arg(&jpeg_thumbnail_path)
        .output()
    {
        if output.status.success() {
            eprintln!("✅ Создана большая миниатюра HEIC через ImageMagick: {}", heic_path.display());

            // Теперь преобразуем JPEG в квадратную иконку 50x50 с центрированием
            let final_icon_path = _thumbnail_path.with_extension("png");
            match create_marker_icon(&jpeg_thumbnail_path, &final_icon_path) {
                Ok(()) => {
                    // Удаляем временный JPEG файл
                    let _ = std::fs::remove_file(&jpeg_thumbnail_path);
                    return Ok(Some(final_icon_path));
                }
                Err(_) => {
                    // Если не удалось создать квадратную иконку, возвращаем JPEG как есть
                    return Ok(Some(jpeg_thumbnail_path));
                }
            }
        }
    }

    // Пытаемся использовать sips (только на macOS)
    #[cfg(target_os = "macos")]
    {
        let sips_thumbnail_path = _thumbnail_path.with_extension("temp.jpg");
        if let Ok(output) = std::process::Command::new("sips")
            .arg("-Z")
            .arg(&(MARKER_SIZE * 4).to_string()) // Создаем большой квадрат 200x200
            .arg(heic_path)
            .arg("--out")
            .arg(&sips_thumbnail_path)
            .output()
        {
            if output.status.success() {
                eprintln!("✅ Создана большая миниатюра HEIC через sips: {}", heic_path.display());

                // Теперь преобразуем JPEG в квадратную иконку 50x50 с центрированием
                let final_icon_path = _thumbnail_path.with_extension("png");
                match create_marker_icon(&sips_thumbnail_path, &final_icon_path) {
                    Ok(()) => {
                        // Удаляем временный JPEG файл
                        let _ = std::fs::remove_file(&sips_thumbnail_path);
                        return Ok(Some(final_icon_path));
                    }
                    Err(_) => {
                        // Если не удалось создать квадратную иконку, возвращаем JPEG как есть
                        return Ok(Some(sips_thumbnail_path));
                    }
                }
            }
        }
    }

    Ok(None) // Не удалось создать миниатюру
}

/// Создает информационную заглушку для HEIC файла
fn create_info_thumbnail(heic_path: &Path, thumbnail_path: &Path) -> Result<()> {
    use std::io::Write;

    // Создаем простое изображение-заглушку с информацией о файле
    let filename = heic_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.heic");

    // Используем библиотеку image для создания заглушки
    let img = image::RgbImage::from_fn(MARKER_SIZE, MARKER_SIZE, |x, y| {
        // Создаем градиентный фон
        let r = (x * 255 / MARKER_SIZE) as u8;
        let g = (y * 255 / MARKER_SIZE) as u8;
        let b = 200;
        image::Rgb([r, g, b])
    });

    let mut dynamic_img = image::DynamicImage::ImageRgb8(img);

    // Добавляем текстовую информацию (просто сохраняем с метаданными)
    let output_format = image::ImageFormat::Jpeg;
    let mut output_file = std::fs::File::create(thumbnail_path)?;

    dynamic_img.write_to(&mut output_file, output_format)?;

    eprintln!("📝 Создана информационная миниатюра для HEIC: {}", filename);
    Ok(())
}

// HTTP Server functionality
use axum::{
    extract::{State, Query},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
    body::Body,
};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;

#[derive(Clone)]
struct AppState {
    photo_count: usize,
}

#[derive(Deserialize)]
struct ConvertHeicQuery {
    filename: String,
}

async fn start_http_server(photo_count: usize) -> Result<()> {
    let state = AppState { photo_count };

    let app = Router::new()
        .route("/", get(serve_map_html))
        .route("/geodata.js", get(serve_geodata))
        .route("/convert-heic", get(convert_heic_to_jpeg))
        .nest_service("/photos", ServeDir::new("photos"))
        .nest_service("/.thumbnails", ServeDir::new(".thumbnails"))
        .layer(
            ServiceBuilder::new()
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                )
        )
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    println!("\n🌐 Сервер запущен на: http://localhost:8080");
    println!("📸 Для просмотра карты откройте: http://localhost:8080");
    println!("⏹️  Для остановки сервера нажмите Ctrl+C или введите 'Q'");

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    println!("👋 Сервер остановлен");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn serve_map_html(State(state): State<AppState>) -> Html<String> {
    Html(MAP_HTML_TEMPLATE.to_string())
}

async fn serve_geodata(State(state): State<AppState>) -> impl IntoResponse {
    match std::fs::read_to_string("geodata.js") {
        Ok(content) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/javascript")
                .header(header::CACHE_CONTROL, "no-cache")
                .body(Body::from(content))
                .unwrap()
        }
        Err(_) => {
            let error_json = format!(
                "var photoData = {{\"error\": \"Геоданные не найдены. Запустите обработку фотографий.\"}};"
            );
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "application/javascript")
                .body(error_json.into())
                .unwrap()
        }
    }
}

async fn convert_heic_to_jpeg(
    Query(query): Query<ConvertHeicQuery>,
) -> impl IntoResponse {
    let photos_dir = Path::new("/Users/dmitriiromanov/claude/photomap/photos");
    let heic_path = photos_dir.join(&query.filename);

    // Проверяем, что файл существует и это HEIC
    if !heic_path.exists() {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/plain")
            .body("HEIC file not found".into())
            .unwrap();
    }

    let ext = heic_path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if !matches!(ext.as_str(), "heic" | "heif") {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(header::CONTENT_TYPE, "text/plain")
            .body("Not a HEIC file".into())
            .unwrap();
    }

    // Генерируем путь для JPEG версии
    let jpeg_filename = query.filename
        .trim_end_matches(".HEIC")
        .trim_end_matches(".heic")
        .trim_end_matches(".HEIF")
        .trim_end_matches(".heif");
    let jpeg_filename = format!("{}_popup.jpg", jpeg_filename);
    let jpeg_path = Path::new(THUMBNAIL_DIR).join(&jpeg_filename);

    // Если JPEG уже существует, возвращаем его
    if jpeg_path.exists() {
        match std::fs::read(&jpeg_path) {
            Ok(jpeg_data) => {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "image/jpeg")
                    .header(header::CACHE_CONTROL, "public, max-age=31536000")
                    .body(Body::from(jpeg_data))
                    .unwrap();
            }
            Err(_) => {
                // Не можем прочитать существующий файл, продолжаем с конвертацией
            }
        }
    }

    // Конвертируем HEIC в JPEG
    if create_heic_jpeg_for_popup(&heic_path, &jpeg_path).is_ok() {
        match std::fs::read(&jpeg_path) {
            Ok(jpeg_data) => {
                eprintln!("✅ Ленивая конвертация HEIC -> JPEG: {}", query.filename);
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "image/jpeg")
                    .header(header::CACHE_CONTROL, "public, max-age=31536000")
                    .body(Body::from(jpeg_data))
                    .unwrap();
            }
            Err(_) => {
                // Не можем прочитать созданный файл
            }
        }
    }

    // Если все попытки неудачны, возвращаем ошибку
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, "text/plain")
        .body("Failed to convert HEIC to JPEG".into())
        .unwrap()
}

// Function to wait for user input for shutdown
async fn wait_for_shutdown_input() {
    use std::io::{self, Write};

    loop {
        print!("➡️  Введите 'Q' для выхода: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim().to_uppercase() == "Q" {
            println!("\n🛑 Остановка сервера...");
            break;
        }
    }
}