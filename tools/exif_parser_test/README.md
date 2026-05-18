# Exif Parser Test Tool 🕵️‍♂️

**Минимальная тестовая версия GPS парсера из PhotoMap** для валидации и отладки.

## Назначение

Этот инструмент проверяет **корректность работы GPS парсера** основного приложения PhotoMap. 

**Что проверяется:**
1. ✅ **Coverage (Покрытие)** - находим ли GPS там где он есть?
2. ✅ **Accuracy (Точность)** - правильно ли читаем координаты?

**Как работает:**
- Использует **идентичный код парсера** из PhotoMap (та же библиотека `kamadak-exif`)
- Сравнивает результаты с **exiftool** (золотой стандарт, 99.99% точность)
- Выявляет файлы где наш парсер ошибается
- После исправления багов → изменения переносятся в основной проект PhotoMap

> **Важно:** Это НЕ самостоятельное приложение, а **тестовый стенд** для проверки парсера.

## Требования

### ⚠️ ОБЯЗАТЕЛЬНО: exiftool

Утилита **не будет работать без exiftool!**

**Установка на Windows:**

**🎯 Вариант 1: Portable (Рекомендуется)**
1. Скачайте с [exiftool.org](https://exiftool.org/) → "Windows Executable"
2. Распакуйте ZIP-архив
3. Переименуйте `exiftool(-k).exe` → `exiftool.exe`
4. Положите **рядом с** `exif_parser_test.exe`:
   ```
   C:\Downloads\
     ├── exif_parser_test.exe
     ├── exiftool.exe           ← скопируйте
     └── exiftool_files\        ← скопируйте
   ```
5. Готово! Никаких PATH не нужно.

**Вариант 2: Системная установка**
- Положите `exiftool.exe` и `exiftool_files\` в `C:\Windows\` (требует прав админа)
- Или добавьте в PATH

**Проверка:** откройте CMD и введите `exiftool -ver`


**Установка на macOS:**
```bash
brew install exiftool
```

**Установка на Linux:**
```bash
sudo apt install libimage-exiftool-perl  # Debian/Ubuntu
# или
sudo pacman -S perl-image-exiftool       # Arch
```

## Скачать готовый .exe (Windows)

**Не нужна компиляция!** Скачайте собранную версию:

1. Перейдите в [Actions → Exif Parser Test](../../actions/workflows/build-exif-parser-test.yml)
2. Нажмите "Run workflow" → выберите `main` → запустите
3. Подождите 5-10 минут
4. Скачайте artifact `exif_parser_test_windows-x64`
5. Распакуйте и запустите `exif_parser_test_windows-x64.exe`

> **Не забудьте установить exiftool!** (см. выше)

## Сборка из исходников

```bash
cd tools/exif_parser_test
cargo build --release
cargo run --release
```

## Использование

1. **Запустите программу** (двойной клик на .exe или `cargo run --release`)
2. **Выберите папку** с фотографиями в диалоге
3. **Дождитесь завершения** - будет показан прогресс
4. **Проверьте результаты:**

```
✅ Scan complete.
Total processed: 100000
Missing GPS (we failed, exiftool succeeded): 5  ← не нашли GPS
Accuracy issues (coordinates mismatch): 2       ← нашли неправильно
```

## Выходные файлы

### 📄 failures.txt
Файлы где **exiftool нашел GPS**, а наш парсер нет:
```
/path/to/photo1.jpg
/path/to/photo2.heic
```
→ **Проблема:** парсер пропускает валидные GPS данные

### 📄 accuracy_issues.txt  
Файлы где координаты **не совпадают** с exiftool:
```
/path/to/photo.jpg | Our: (48.8566, 2.3522) | exiftool: (48.8570, 2.3525) | Diff: (0.0004, 0.0003)
```
→ **Проблема:** парсер читает координаты неправильно (tolerance: 0.0001° ≈ 11 метров)

### 📁 JPG for checks/
Автоматические копии проблемных файлов для ручного анализа.

## Что проверяется

**Парсер в этой утилите = 100% код из PhotoMap:**
- ✅ JPEG/TIFF через `kamadak-exif`
- ✅ HEIC/HEIF/AVIF через `libheif-rs` + `kamadak-exif`
- ✅ Поиск GPS в любом IFD (PRIMARY, GPS, и др.)
- ✅ Обработка "сломанного" EXIF (`continue_on_error`)
- ✅ Xiaomi HEIC bug (JPEG с расширением .heic)

**Валидация:**
1. Наш парсер извлекает GPS
2. exiftool извлекает GPS
3. Сравниваем результаты:
   - Оба не нашли → OK (GPS просто нет)
   - Оба нашли одинаковое → OK
   - exiftool нашел, мы нет → **FAILURE** (coverage)
   - Нашли разные координаты → **ACCURACY ISSUE** (precision)

## Workflow разработки

```
1. Находим проблемные файлы через test tool
      ↓
2. Анализируем причину (камера, формат, структура EXIF)
      ↓
3. Исправляем парсер в tools/exif_parser_test/src/main.rs
      ↓
4. Тестируем → проблема устранена?
      ↓
5. Переносим исправления в PhotoMap/src/exif_parser/
      ↓
6. Собираем новую версию PhotoMap
```

## Известные исправления

- ✅ **Samsung SM-G780G** - GPS в нестандартном IFD
- ✅ **Xiaomi HEIC bug** - JPEG с расширением .heic  
- ✅ **Lightroom EXIF** - "битый" EXIF с ошибками
- ✅ **Apple HEIC** - стандартный формат от iPhone

## Технические детали

**Зависимости:**
- `kamadak-exif` - парсинг EXIF (та же версия что в PhotoMap)
- `libheif-rs` - HEIC поддержка (та же версия)
- `walkdir` - рекурсивный обход папок
- `rfd` - нативный диалог выбора папки
- `anyhow` - обработка ошибок

**Производительность:**
- ~1-2 секунды на 10,000 файлов (без exiftool проверки)
- ~500-1000 минут на 100,000 файлов (с полной валидацией через exiftool)

**Ограничения:**
- Требует установленный exiftool
- Работает только с форматами: JPEG, HEIC, HEIF, AVIF, TIFF
- Не проверяет datetime и другие EXIF поля (только GPS)
