# PhotoMap Critical Solutions Documentation

## 1. 🗂️ Выбор папок через браузер (HTML5 File API)

### Проблема
Нужен кроссплатформенный способ выбора папок, который работает без внешних зависимостей и сложной архитектуры.

### ✅ РЕШЕНИЕ: HTML5 File API с webkitdirectory

#### Архитектура
1. **Browser Native**: Использует HTML5 File API с атрибутом `webkitdirectory`
2. **JavaScript Integration**: JavaScript функция вызывает системный диалог
3. **Server Communication**: Путь отправляется на сервер через REST API
4. **Automatic Processing**: Обработка запускается сразу после выбора папки

#### HTML Template (`src/html_template.rs`)
```html
<!-- Скрытый input для выбора папки -->
<input type="file" id="folder-input-hidden" style="display: none;" webkitdirectory directory multiple>

<!-- Кнопка для вызова диалога -->
<button id="browse-button" onclick="browseAndProcessFolder()">📁 Обзор</button>
```

#### JavaScript Implementation
```javascript
async function browseAndProcessFolder() {
    // Создаем Promise для обработки выбора папки
    const folderSelection = new Promise((resolve, reject) => {
        const hiddenInput = document.getElementById('folder-input-hidden');

        hiddenInput.onchange = function(e) {
            const files = e.target.files;
            if (files && files.length > 0) {
                // Извлекаем имя папки из первого файла
                const firstFile = files[0];
                const fullPath = firstFile.webkitRelativePath;
                const folderPath = fullPath.split('/')[0];
                resolve(folderPath);
            } else {
                reject(new Error('Folder selection cancelled'));
            }
        };

        hiddenInput.click();
    });

    try {
        // Ждем выбора папки
        const folderPath = await folderSelection;

        // Отправляем путь на сервер
        const response = await fetch('/api/set-folder', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ folder_path: folderPath })
        });

        // Запускаем обработку
        await fetch('/api/process', { method: 'POST' });
    } catch (error) {
        // Обработка ошибок или отмены
        console.error('Folder selection error:', error);
    }
}
```

#### Server Integration (`src/server.rs`)
```rust
// API endpoint для установки пути папки
pub async fn set_folder(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>
) -> Result<Json<serde_json::Value>, StatusCode> {
    let folder_path = payload.get("folder_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            Json(serde_json::json!({
                "status": "error",
                "message": "No folder_path provided"
            }))
        })?;

    // Сохраняем в настройки
    let mut settings = state.settings.lock().unwrap();
    settings.last_folder = Some(folder_path.to_string());
    let _ = settings.save();

    Ok(Json(serde_json::json!({
        "status": "success",
        "folder_path": folder_path
    })))
}
```

#### Ключевые моменты
- **Zero dependencies**: Работает без внешних программ
- **Cross-platform**: Работает во всех современных браузерах
- **Native UX**: Использует системные диалоги выбора папки
- **Automatic processing**: Запускает обработку сразу после выбора
- **Error handling**: Корректно обрабатывает отмену выбора

#### Результат работы
```
✅ Папка выбрана: Photos
🔍 Setting folder from browser dialog
✅ Folder set: Photos
✅ Обработка запущена: Photos
```

---

## 2. 📱 HEIC Конвертация: Предотвращение квадратных миниатюр

### Проблема
HEIC файлы конвертировались в квадратные миниатюры (50x50px) вместо сохранения пропорций.

### ❌ НЕПРАВИЛЬНЫЕ ПАРАМЕТРЫ ImageMagick
```rust
// НЕПРАВИЛЬНО - делает все квадратными
"thumbnail" => vec![
    &photo.file_path,
    "-resize", "60x60^",     // ^ заставляет быть квадратным
    "-gravity", "center",
    "-extent", "60x60",      // Pad до квадрата
    "-quality", "80",
    "jpg:-"
]
```

### ✅ ПРАВИЛЬНЫЕ ПАРАМЕТРЫ ImageMagick
```rust
// ПРАВИЛЬНО - сохраняет пропорции
"thumbnail" => vec![
    &photo.file_path,
    "-resize", "60x60>",      // > только если больше, сохраняет пропорции
    "-quality", "80",
    "jpg:-"
]

// ПРАВИЛЬНО - для маркеров
"marker" => vec![
    &photo.file_path,
    "-resize", "40x40>",       // > только если больше, сохраняет пропорции
    "-quality", "80",
    "jpg:-"
]
```

#### Ключевые параметры ImageMagick

| Размер | Параметр | Результат |
|--------|----------|---------|
| 40x40> | Только если оригинал больше 40px, сохраняет пропорции |
| 60x60> | Только если оригинал больше 60px, сохраняет пропорции |
| 40x40^ | Заставляет быть 40x40, искажает пропорции (НЕ ИСПОЛЬЗОВАТЬ) |
| 60x60^ | Заставляет быть 60x60, искажает пропорции (НЕ ИСПОЛЬЗОВАТЬ) |

#### Размеры сервера (выжные значения)
```rust
// Маркеры: 40x40px (отображаются как 40px)
create_marker_icon_in_memory() -> 40x40px

// Миниатюры: 60x60px (отображаются как 60px)
create_thumbnail_in_memory() -> 60x60px
```

#### Размеры клиента (выжные значения)
```javascript
// Клиентские настройки в html_template.js
markerOptions: {
    iconSize: [40, 40],        // 40px маркеры
    iconCreateFunction: function(cluster) {
        return L.divIcon({
            html: `<div style="width:40px;height:40px;overflow:hidden;border-radius:50%;background-image:url('${iconUrl}');background-size:cover;background-position:center;"></div>`,
            iconSize: [40, 40],
            className: 'custom-marker'
        });
    }
},

// useThumbnail: false -> маркеры 40px
// useThumbnail: true -> миниатюры 60px
```

---

## 3. 🏗️ Архитектурные решения и рефакторинг

### Текущая структура проекта
```
photomap/
├── src/
│   ├── main.rs              # Основная логика и запуск
│   ├── server.rs            # HTTP API эндпоинты
│   ├── database.rs          # SQLite операции
│   ├── folder_picker.rs     # Устаревший модуль выбора папок
│   ├── image_processing.rs  # Обработка изображений (ВАЖНО: HEIC параметры)
│   ├── html_template.rs     # HTML и JavaScript с webkitdirectory
│   ├── settings.rs          # INI настройки
│   └── exif_parser.rs       # EXIF парсинг
└── photos/                  # Директория с фотографиями
```

### Рекомендации по рефакторингу
1. **Убрать unused imports** (dirs, start_server, kill_photomap_processes)
2. **Разделить server.rs** на модули:
   - `api_endpoints.rs` - HTTP обработчики
   - `sse_events.rs` - Server-Sent Events
   - `app_state.rs` - AppState и related
3. **Вынести константы** в отдельный `constants.rs`:
   - Размеры изображений (40px, 60px)
   - Порты (3001)
   - Имена файлов
4. **Создать модуль `heic_processing.rs`** для HEIC логики

### Критически важные константы
```rust
// constants.rs
pub const MARKER_SIZE: u32 = 40;
pub const THUMBNAIL_SIZE: u32 = 60;
pub const DEFAULT_PORT: u16 = 3001;
pub const HEIC_MARKER_SIZE: &str = "40x40>";
pub const HEIC_THUMBNAIL_SIZE: &str = "60x60>";
```

---

## 4. 🚫 Запрещенные подходы (никогда не использовать)

1. **Параметры ImageMagick с `^` для принудительного квадрата**
2. **Изменение размеров на клиенте без синхронизации с сервером**
3. **Типа `Vec<u8>` без `Cursor` для image encoding**

---

## 5. ✅ Выжные проверки перед каждым запуском

1. **Проверить HEIC конвертацию**:
   ```bash
   # Убедиться что в image_processing.rs используются параметры "40x40>" и "60x60>"
   ```

2. **Проверить сервер**:
   ```bash
   cargo run
   # Открыть http://127.0.0.1:3001
   # Нажать кнопку "Обзор" для проверки выбора папки
   ```

3. **Проверить миниатюры**:
   - Открыть HEIC файл в браузере
   - Убедиться что пропорции сохранены

4. **Проверить API**:
   ```bash
   curl http://127.0.0.1:3001/api/photos
   # Должен вернуть список фотографий с GPS
   ```