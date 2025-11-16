# PhotoMap Refactoring Recommendations

## 🎯 Цели рефакторинга
1. Уменьшить размер файлов для лучшей поддержки
2. Улучшить читаемость и поддерживаемость кода
3. Устранить неиспользуемые зависимости
4. Вынести константы и магические числа

## 📁 Текущая структура
```
photomap/
├── src/
│   ├── main.rs              (490 строк) - Основная логика, запуск, обработка фотографий
│   ├── server.rs            (482 строки) - HTTP API, SSE, обработчики
│   ├── folder_picker.rs     (185 строк) - Выбор папок, external helper
│   ├── image_processing.rs  (335 строк) - HEIC/JPEG обработка, ImageMagick
│   ├── html_template.rs     (650+ строк) - HTML + JavaScript (очень большой)
│   ├── database.rs          (150 строк) - SQLite операции
│   ├── settings.rs          (120 строк) - INI настройки
│   ├── exif_parser.rs      (280 строк) - EXIF парсинг
│   └── port_manager.rs      (110 строк) - Управление портами
├── folder_dialog_helper/     # Helper для RFD (критически важный)
└── photos/                   # Папка с фотографиями
```

## 🔧 Рекомендации по рефакторингу

### 1. Разделить server.rs (сейчас 482 строки)
```rust
src/
├── server/
│   ├── mod.rs               // re-export всех модулей
│   ├── app_state.rs         // AppState, FolderRequestHandler
│   ├── api_endpoints.rs     // HTTP обработчики
│   ├── sse_events.rs        // Server-Sent Events
│   └── responses.rs         // Типы ответов
```

### 2. Разделить main.rs (сейчас 490 строк)
```rust
src/
├── core/
│   ├── mod.rs               // re-export
│   ├── photo_processor.rs  // Основная логика обработки
│   └── app_runner.rs       // Запуск приложения
```

### 3. Уменьшить html_template.rs (сейчас 650+ строк)
```rust
src/
├── web/
│   ├── mod.rs
│   ├── html.rs             // HTML шаблон (без JS)
│   ├── js.rs               // JavaScript логика
│   ├── css.rs              // CSS стили
│   └── components.rs       // React-like компоненты
```

### 4. Создать constants.rs
```rust
// src/constants.rs
pub const MARKER_SIZE: u32 = 40;
pub const THUMBNAIL_SIZE: u32 = 60;
pub const DEFAULT_PORT: u16 = 3001;
pub const MAX_PHOTOS_PER_REQUEST: usize = 1000;
pub const HEIC_MARKER_SIZE: &str = "40x40>";
pub const HEIC_THUMBNAIL_SIZE: &str = "60x60>";

pub const POPUP_WIDTH: u32 = 700;
pub const INFO_PANEL_WIDTH: u32 = 333; // 25% of 1333px
```

### 5. Создать heic_processing.rs
```rust
// src/heic_processing.rs
pub mod heic_converter {
    pub const MARKER_PARAMS: &[&str] = &["-resize", "40x40>", "-quality", "80"];
    pub const THUMBNAIL_PARAMS: &[&str] = &["-resize", "60x60>", "-quality", "80"];
    // ... остальная логика
}
```

## 🚫 Что НЕ рефакторить (критически важное)

### 1. folder_dialog_helper/
- **НЕ трогать** - это рабочее решение threading проблемы
- Оставить как отдельный Cargo project
- Критически важен для macOS

### 2. image_processing.rs HEIC параметры
```rust
// НЕ МЕНЯТЬ эти параметры!
"marker" => vec![..., "-resize", "40x40>", ...]
"thumbnail" => vec![..., "-resize", "60x60>", ...]
```

### 3. База данных SQLite
- Оставить текущую схему
- `relative_path` поле критически важно

### 4. SSE и real-time обновления
- Оставить как есть
- Работает хорошо

## 📋 Очередь рефакторинга (приоритет)

### Высокий приоритет
1. ✅ Создать `constants.rs`
2. ⏳ Убрать unused imports
3. ⏳ Разделить `server.rs`

### Средний приоритет
4. ⏳ Вынести HEIC логику в отдельный модуль
5. ⏳ Разделить `main.rs`

### Низкий приоритет
6. ⏳ Рефакторинг `html_template.rs`
7. ⏳ Улучшить error handling

## 🔧 Конкретные изменения

### 1. Убрать unused imports (немедленно)
```bash
# В src/settings.rs удалить:
use dirs;  # ❌ Unused

# В src/main.rs удалить:
use server::{AppState, start_server, start_server_with_port}; // ❌ start_server unused
use port_manager::{find_available_port, kill_processes_using_port, kill_photomap_processes}; // ❌ kill_photomap_processes unused
```

### 2. Создать constants.rs
```rust
// src/constants.rs
pub const MARKER_SIZE_PX: u32 = 40;
pub const THUMBNAIL_SIZE_PX: u32 = 60;
pub const DEFAULT_PORT: u16 = 3001;

pub struct ImageSizes {
    pub marker: (u32, u32),
    pub thumbnail: (u32, u32),
}

impl Default for ImageSizes {
    fn default() -> Self {
        Self {
            marker: (MARKER_SIZE_PX, MARKER_SIZE_PX),
            thumbnail: (THUMBNAIL_SIZE_PX, THUMBNAIL_SIZE_PX),
        }
    }
}
```

### 3. Переместить HEIC логику
```rust
// src/heic_processing.rs
use crate::constants::*;

pub fn get_heic_conversion_params(size_type: &str) -> Vec<String> {
    let base_cmd = vec!["convert".to_string()];

    let (resize_param, quality_param) = match size_type {
        "marker" => (HEIC_MARKER_SIZE, "80"),
        "thumbnail" => (HEIC_THUMBNAIL_SIZE, "80"),
        "full" => ("1024x1024>", "90"),
        _ => ("60x60>", "80"),
    };

    base_cmd.extend_from_slice(&[resize_param.to_string(), quality_param.to_string()]);
    base_cmd
}
```

## 📊 Ожидаемые результаты

### После рефакторинга:
- **main.rs**: ~200 строк (сейчас 490)
- **server.rs**: ~200 строк (сейчас 482)
- **html_template.rs**: ~300 строк (сейчас 650+)
- **Улучшение читаемости**: ✅
- **Устранение дублирования**: ✅
- **Конфигурируемость**: ✅

### Сохранить:
- ✅ Функциональность RFD helper
- ✅ HEIC конвертация параметры
- ✅ SSE и real-time обновления
- ✅ SQLite база данных

---

## 🎯 Следующие шаги
1. Создать `constants.rs`
2. Убрать unused imports (cargo fix)
3. Постепенно рефакторить модули
4. Тестировать после каждого изменения