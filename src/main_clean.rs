use anyhow::{Context, Result};
use ignore::Walk;
use exif::{In, Reader, Tag, Value};
use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

// Structure to store metadata for each photo.
// `Serialize` is needed for JSON conversion.
#[derive(Serialize, Debug)]
struct ImageMetadata {
    filename: String,
    path: String,       // Relative path to original file
    thumbnail: String,  // Relative path to thumbnail
    lat: f64,
    lng: f64,
    datetime: String,   // Date and time from EXIF (DD.MM.YYYY HH:MM)
}

const THUMBNAIL_DIR: &str = ".thumbnails";
const THUMBNAIL_SIZE: u32 = 700;
const OUTPUT_FILE: &str = "geodata.js";
const MAP_HTML_FILE: &str = "map.html";

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
        }
        .leaflet-popup-content {
            width: 720px !important;
            padding: 12px !important;
            margin: 0 !important;
        }
        .leaflet-popup-content p {
            margin: 8px 0 0 0;
            padding: 0;
        }
        .popup-date {
            font-size: 0.9em;
            color: #666;
            margin-top: 8px;
        }
        .popup-filename {
            margin-bottom: 8px;
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

            photoData.forEach(function(photo) {
                // Создаем иконку маркера из миниатюры
                const customIcon = L.icon({
                    iconUrl: photo.thumbnail,
                    iconSize: [50, 50],
                    iconAnchor: [25, 25],
                    popupAnchor: [0, -25],
                    className: 'custom-marker' // для кастомизации через CSS
                });

                // Создаем маркер
                const marker = L.marker([photo.lat, photo.lng], { icon: customIcon });

                // Создаем содержимое для всплывающего окна (popup)
                // Сначала показываем дату съемки, затем имя файла (по просьбе пользователя)
                const popupContent = `
                    <img src="${photo.path}" alt="${photo.filename}" class="popup-image">
                    <p class="popup-date">${photo.datetime}</p>
                    <p class="popup-filename"><strong>${photo.filename}</strong></p>
                `;
                marker.bindPopup(popupContent);

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
    </script>

</body>
</html>"#;

fn main() -> Result<()> {
    println!("🗺️  PhotoMap Processor запускается...");

    // 0. Создаем map.html если его еще нет
    if !std::path::Path::new(MAP_HTML_FILE).exists() {
        println!("📄 Создаю map.html...");
        create_map_html()?;
        println!("✅ map.html создан в текущей директории: {}", MAP_HTML_FILE);
    } else {
        println!("📄 map.html уже существует в текущей директории: {}", MAP_HTML_FILE);
    }

    // 1. Создаем папку для миниатюр, если ее нет
    fs::create_dir_all(THUMBNAIL_DIR)
        .with_context(|| format!("Не удалось создать папку для миниатюр: {}", THUMBNAIL_DIR))?;

    // 2. Получаем список всех файлов в текущем каталоге и подпапках
    println!("🔍 Сканирование текущей директории и подпапок...");
    let current_dir = std::env::current_dir()?;
    println!("📂 Текущая директория: {}", current_dir.display());

    // Создаем walker для текущей директории с ограничением
    let walker = Walk::new(&current_dir);
    let files: Vec<PathBuf> = walker
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|e| {
            // Проверяем, что файл находится в текущей директории или ее подпапках
            e.path().starts_with(&current_dir)
        })
        .filter(|e| {
            // Исключаем системные директории и скрытые файлы
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
    println!("✅ Найдено {} файлов в текущей директории. Начинаю обработку...", files.len());

    // 3. Обрабатываем файлы параллельно с помощью Rayon
    let photo_data: Vec<ImageMetadata> = files
        .par_iter() // <-- Магия параллелизма!
        .filter_map(|path| process_file(path).ok()) // Отфильтровываем файлы, которые не удалось обработать
        .collect();

    println!("✅ Обработка завершена. Найдено {} фотографий с GPS-данными.", photo_data.len());

    // 4. Записываем результат в geodata.js
    write_geodata_js(&photo_data)?;

    println!(
        "🎉 Готово! Данные сохранены в файле '{}' в текущей директории.",
        OUTPUT_FILE
    );
    println!("🌐 Для просмотра карты откройте в браузере файл: {}", std::env::current_dir()?.join(MAP_HTML_FILE).display());
    println!("💡 Или выполните команду: open {}", MAP_HTML_FILE);

    // Ждем ввода пользователя перед закрытием
    pause_and_wait_for_input()?;

    Ok(())
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

    let thumbnail_path = generate_thumbnail_path(path)?;
    
    // Для HEIC/AVIF используем заглушку для миниатюр, для остальных - открываем файл
    
    let mut final_thumbnail_path = thumbnail_path.clone();

    if is_heif {
        // Умное создание миниатюры для HEIC
        match create_heic_thumbnail(path, &thumbnail_path)? {
            Some(heic_thumbnail_path) => {
                final_thumbnail_path = heic_thumbnail_path;
            }
            None => {
                // Если не удалось создать миниатюру, создаем информационную заглушку
                create_info_thumbnail(path, &thumbnail_path)?;
            }
        }
    } else {
        create_thumbnail(path, &thumbnail_path)?;
    }

    // --- Формирование результата ---
    let metadata = ImageMetadata {
        filename: filename.to_string(),
        path: path.to_string_lossy().into_owned(),
        thumbnail: final_thumbnail_path.to_string_lossy().into_owned(),
        lat,
        lng,
        datetime,
    };

    Ok(metadata)
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

/// Создает миниатюру для изображения.
fn create_thumbnail(source_path: &Path, thumbnail_path: &Path) -> Result<()> {
    let mut img = image::open(source_path)
        .with_context(|| format!("Не удалось открыть изображение: {:?}", source_path))?;

    // Применяем EXIF-ориентацию
    img = apply_exif_orientation(source_path, img)?;

    // Используем thumbnail() для сохранения пропорций
    let thumbnail = img.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);
    thumbnail.save(thumbnail_path)?;
    Ok(())
}

/// Создает миниатюру из уже декодированного image::DynamicImage (для HEIC/AVIF).
#[allow(dead_code)]
fn create_thumbnail_from_dynamic_image(img: &image::DynamicImage, thumbnail_path: &Path) -> Result<()> {
    // Используем thumbnail() для сохранения пропорций
    let thumbnail = img.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);
    thumbnail.save(thumbnail_path)?;
    Ok(())
}

/// Генерирует уникальный и безопасный путь для миниатюры.
fn generate_thumbnail_path(original_path: &Path) -> Result<PathBuf> {
    let filename = original_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::Error::msg("Некорректное имя файла"))?;
    let safe_filename = filename.replace('/', "_").replace('\\', "_");
    Ok(Path::new(THUMBNAIL_DIR).join(safe_filename))
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
    // Создаем JPEG миниатюру для HEIC файла
    let jpeg_thumbnail_path = _thumbnail_path.with_extension("jpg");

    if let Ok(output) = std::process::Command::new("magick")
        .arg(heic_path)
        .arg("-resize")
        .arg(&format!("{}x{}", THUMBNAIL_SIZE, THUMBNAIL_SIZE))
        .arg("-quality")
        .arg("80")
        .arg(&jpeg_thumbnail_path)
        .output()
    {
        if output.status.success() {
            eprintln!("✅ Создана миниатюра HEIC через ImageMagick: {}", heic_path.display());
            return Ok(Some(jpeg_thumbnail_path));
        }
    }

    // Пытаемся использовать sips (только на macOS)
    #[cfg(target_os = "macos")]
    {
        let sips_thumbnail_path = _thumbnail_path.with_extension("jpg");
        if let Ok(output) = std::process::Command::new("sips")
            .arg("-Z")
            .arg(&THUMBNAIL_SIZE.to_string())
            .arg(heic_path)
            .arg("--out")
            .arg(&sips_thumbnail_path)
            .output()
        {
            if output.status.success() {
                eprintln!("✅ Создана миниатюра HEIC через sips: {}", heic_path.display());
                return Ok(Some(sips_thumbnail_path));
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
    let img = image::RgbImage::from_fn(THUMBNAIL_SIZE, THUMBNAIL_SIZE, |x, y| {
        // Создаем градиентный фон
        let r = (x * 255 / THUMBNAIL_SIZE) as u8;
        let g = (y * 255 / THUMBNAIL_SIZE) as u8;
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

fn pause_and_wait_for_input() {
    println!("
✋ Press any key to exit...");
    let _ = std::io::stdin().read_line(&mut String::new());
}

fn main() {
    if let Err(e) = run() {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    pause_and_wait_for_input();
    Ok(())
}
