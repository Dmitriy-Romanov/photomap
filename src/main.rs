use anyhow::{Context, Result};
use ignore::Walk;
use exif::{In, Reader, Tag, Value};
use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

// Структура для хранения метаданных о каждой фотографии.
// `Serialize` нужен для преобразования в JSON.
#[derive(Serialize, Debug)]
struct ImageMetadata {
    filename: String,
    path: String,       // Относительный путь к оригинальному файлу
    thumbnail: String,  // Относительный путь к миниатюре
    lat: f64,
    lng: f64,
    datetime: String,   // Дата и время съемки из EXIF (ДД.ММ.ГГГГ ЧЧ:ММ)
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
        println!("✅ map.html создан.");
    }

    // 1. Создаем папку для миниатюр, если ее нет
    fs::create_dir_all(THUMBNAIL_DIR)
        .with_context(|| format!("Не удалось создать папку для миниатюр: {}", THUMBNAIL_DIR))?;

    // 2. Получаем список всех файлов в текущем каталоге
    println!("🔍 Сканирование каталога...");
    let walker = Walk::new("./");
    let files: Vec<PathBuf> = walker
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|e| e.file_type().map_or(false, |ft| ft.is_file()))
        .map(|e| e.into_path())
        .collect();
    println!("✅ Найдено {} файлов. Начинаю обработку...", files.len());

    // 3. Обрабатываем файлы параллельно с помощью Rayon
    let photo_data: Vec<ImageMetadata> = files
        .par_iter() // <-- Магия параллелизма!
        .filter_map(|path| process_file(path).ok()) // Отфильтровываем файлы, которые не удалось обработать
        .collect();

    println!("✅ Обработка завершена. Найдено {} фотографий с GPS-данными.", photo_data.len());

    // 4. Записываем результат в geodata.js
    write_geodata_js(&photo_data)?;

    println!(
        "🎉 Готово! Данные сохранены в '{}'. Откройте map.html в браузере.",
        OUTPUT_FILE
    );

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
    
    // Базовый список поддерживаемых форматов
    let supported_formats = if cfg!(feature = "heif") {
        ["jpg", "jpeg", "png", "tiff", "tif", "webp", "bmp", "gif", "heic", "heif", "avif"].iter().map(|s| *s).collect::<Vec<_>>()
    } else {
        ["jpg", "jpeg", "png", "tiff", "tif", "webp", "bmp", "gif"].iter().map(|s| *s).collect::<Vec<_>>()
    };
    
    if !supported_formats.contains(&ext.as_deref().unwrap_or("")) {
        let formats = if cfg!(feature = "heif") {
            "JPG, PNG, WebP, TIFF, BMP, GIF, HEIC, HEIF, AVIF"
        } else {
            "JPG, PNG, WebP, TIFF, BMP, GIF (поддержка HEIC включается с feature 'heif')"
        };
        anyhow::bail!("Файл не является поддерживаемым изображением (поддерживается: {})", formats);
    }

    // --- Извлечение GPS-данных ---
    let file = fs::File::open(path)?;
    let mut bufreader = std::io::BufReader::new(&file);
    let exifreader = Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader)?;

    let lat = get_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?;
    let lng = get_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?;

    if lat.is_none() || lng.is_none() {
        anyhow::bail!("GPS-данные не найдены");
    }
    let lat = lat.unwrap();
    let lng = lng.unwrap();

    // --- Извлечение даты съемки ---
    let datetime = get_datetime_from_exif(&exif).unwrap_or_else(|| "Дата неизвестна".to_string());

    // --- Создание миниатюры ---
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::Error::msg("Некорректное имя файла"))?;

    let thumbnail_path = generate_thumbnail_path(path)?;
    create_thumbnail(path, &thumbnail_path)?;

    // --- Формирование результата ---
    let metadata = ImageMetadata {
        filename: filename.to_string(),
        path: path.to_string_lossy().into_owned(),
        thumbnail: thumbnail_path.to_string_lossy().into_owned(),
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

/// Создает миниатюру для изображения.
fn create_thumbnail(source_path: &Path, thumbnail_path: &Path) -> Result<()> {
    let img = image::open(source_path)
        .with_context(|| format!("Не удалось открыть изображение: {:?}", source_path))?;

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
// HEIC/AVIF поддержка (опциональна, включается через feature 'heif')
// ============================================================

#[cfg(feature = "heif")]
/// Декодирует HEIC/AVIF файл в стандартный формат для обработки.
/// Требует feature 'heif' и установленную libheif через vcpkg/system.
fn decode_heif_to_image(path: &Path) -> Result<image::DynamicImage> {
    use libheif_sys::{
        heif_context_alloc, heif_context_free, heif_context_read_from_file,
        heif_context_get_primary_image_handle, heif_image_handle_release,
        heif_decode_image, heif_image_release, heif_colorspace_RGB,
        heif_chroma_interleaved_RGB, heif_image_get_plane_readonly,
        heif_channel_interleaved,
    };
    
    unsafe {
        // Выделяем контекст libheif
        let ctx = heif_context_alloc();
        if ctx.is_null() {
            anyhow::bail!("Не удалось выделить контекст libheif");
        }
        
        // Читаем файл в контекст
        let path_cstr = std::ffi::CString::new(path.to_string_lossy().as_bytes())?;
        let read_result = heif_context_read_from_file(ctx, path_cstr.as_ptr(), std::ptr::null());
        if !read_result.code == 0 { // code 0 = no error
            heif_context_free(ctx);
            anyhow::bail!("Не удалось прочитать HEIF файл: {}", path.display());
        }
        
        // Получаем первичное изображение
        let mut handle = std::ptr::null_mut();
        let handle_result = heif_context_get_primary_image_handle(ctx, &mut handle);
        if !handle_result.code == 0 || handle.is_null() {
            heif_context_free(ctx);
            anyhow::bail!("Не удалось получить основное изображение из HEIF файла");
        }
        
        // Декодируем в RGB
        let mut img = std::ptr::null_mut();
        let decode_result = heif_decode_image(handle, &mut img, heif_colorspace_RGB, heif_chroma_interleaved_RGB, std::ptr::null_mut());
        if !decode_result.code == 0 || img.is_null() {
            heif_image_handle_release(handle);
            heif_context_free(ctx);
            anyhow::bail!("Не удалось декодировать HEIF изображение");
        }
        
        // Получаем данные пикселей
        let mut stride = 0i32;
        let data = heif_image_get_plane_readonly(img, heif_channel_interleaved, &mut stride);
        if data.is_null() {
            heif_image_release(img);
            heif_image_handle_release(handle);
            heif_context_free(ctx);
            anyhow::bail!("Не удалось получить данные пикселей из HEIF");
        }
        
        // TODO: конвертировать raw буфер в image::DynamicImage
        // Это требует получения ширины, высоты и копирования буфера в image::RgbaImage
        // Временно возвращаем ошибку
        heif_image_release(img);
        heif_image_handle_release(handle);
        heif_context_free(ctx);
        
        anyhow::bail!("HEIC декодирование: реализация в разработке");
    }
}

#[cfg(not(feature = "heif"))]
/// Stub для когда feature 'heif' отключена.
fn decode_heif_to_image(_path: &Path) -> Result<image::DynamicImage> {
    anyhow::bail!("HEIC поддержка отключена (включите feature 'heif' в Cargo.toml)")
}