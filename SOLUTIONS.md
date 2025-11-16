# PhotoMap Critical Solutions Documentation

## 1. 🗂️ Системный визард выбора папок (RFD Threading Problem)

### Проблема
На macOS rfd crate не может вызывать системные диалоги из async контекста:
```
You are running RFD in NonWindowed environment, it is impossible to spawn dialog from thread different than main in this env.
```

### ❌ РЕШЕНИЯ, КОТОРЫЕ НЕ РАБОТАЮТ (ЗАПРЕЩЕНЫ)

1. **Прямой вызов rfd в async HTTP handler**
   ```rust
   // НЕ РАБОТАЕТ - падает с паникой
   async fn select_folder() -> Result<Json<FolderResponse>, StatusCode> {
       match rfd::FileDialog::new().pick_folder() {
           // Panic на macOS
       }
   }
   ```

2. **Использование spawn_blocking**
   ```rust
   // НЕ РАБОТАЕТ - та же ошибка
   let result = tokio::task::spawn_blocking(|| {
       rfd::FileDialog::new().pick_folder()
   }).await;
   ```

3. **Channel-based подход без внешнего процесса**
   ```rust
   // НЕ РАБОТАЕТ - все еще в async контексте
   let (tx, rx) = mpsc::channel();
   tokio::spawn(async move {
       rfd::FileDialog::new().pick_folder() // Still panic
   });
   ```

### ✅ РАБОЧЕЕ РЕШЕНИЕ: Внешний Helper Process

#### Архитектура
1. **Helper Program**: `folder_dialog_helper/src/main.rs` - отдельная программа
2. **Process Execution**: `tokio::process::Command` для вызова helper
3. **Channel Communication**: для асинхронной обработки

#### Helper Program (`folder_dialog_helper/src/main.rs`)
```rust
use std::path::PathBuf;

fn main() {
    match rfd::FileDialog::new()
        .set_title("Select folder for PhotoMap")
        .pick_folder()
    {
        Some(path) => {
            println!("{}", path.display()); // Вывод в stdout
        }
        None => {
            std::process::exit(1); // User cancelled
        }
    }
}
```

#### Server Integration (`src/folder_picker.rs`)
```rust
async fn handle_folder_selection_async() -> Option<PathBuf> {
    let helper_path = {
        let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        path.push("folder_dialog_helper");
        path.push("target");
        path.push("release");
        path.push("folder_dialog_helper");
        path
    };

    if helper_path.exists() {
        match tokio::process::Command::new(&helper_path)
            .output()
            .await
        {
            Ok(output) => {
                if output.status.success() {
                    let path_str_owned = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path_str_owned.is_empty() {
                        let selected_path = PathBuf::from(path_str_owned);
                        return Some(selected_path);
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to execute folder dialog helper: {}", e);
            }
        }
    }

    // Fallback к обычным директориям
    // ...
}
```

#### Ключевые моменты
- **Helper runs on main thread** - может использовать rfd без проблем
- **Process isolation** - async контекст не влияет на helper
- **Stdout communication** - простой способ передачи пути обратно
- **Graceful fallback** - если helper недоступен, используются стандартные директории

#### Результат работы
```
🔍 Folder selection requested via API
📁 Received folder request: request_1763232665461
🗂️  Launching external folder dialog helper
🚀 Executing folder dialog helper: /Users/dmitriiromanov/claude/photomap/folder_dialog_helper/target/release/folder_dialog_helper
✅ Folder selected via helper: /Users/dmitriiromanov/Movies/Полиглот. Немецкий с нуля за 16 часов! (2014)
✅ Folder selected: /Users/dmitriiromanov/Movies/Полиглот. Немецкий с нуля за 16 часов! (2014)
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
│   ├── folder_picker.rs     # Выбор папок (ВАЖНО: helper approach)
│   ├── image_processing.rs  # Обработка изображений (ВАЖНО: HEIC параметры)
│   ├── html_template.rs     # HTML и JavaScript
│   ├── settings.rs          # INI настройки
│   └── port_manager.rs      # Управление портами
├── folder_dialog_helper/    # Helper для RFD (ВАЖНО: отделен)
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

1. **Прямой вызов rfd в async контексте**
2. **Параметры ImageMagick с `^` для принудительного квадрата**
3. **Изменение размеров на клиенте без синхронизации с сервером**
4. **Использование spawn_blocking для rfd на macOS**
5. **Типа `Vec<u8>` без `Cursor` для image encoding**

---

## 5. ✅ Выжные проверки перед каждым запуском

1. **Проверить helper program**:
   ```bash
   cd folder_dialog_helper && cargo build --release
   ./target/release/folder_dialog_helper
   ```

2. **Проверить HEIC конвертацию**:
   ```bash
   # Убедиться что в image_processing.rs используются параметры "40x40>" и "60x60>"
   ```

3. **Проверить сервер**:
   ```bash
   cargo run
   curl http://127.0.0.1:3001/api/select-folder
   ```

4. **Проверить миниатюры**:
   - Открыть HEIC файл в браузере
   - Убедиться что пропорции сохранены