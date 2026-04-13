# .gitignore обновлён!

## ✅ Добавлено для предотвращения попадания в репозиторий:

### 🛠️ Development скрипты (не для production):
- `*.sh` - все shell скрипты в корне
- `build.sh`, `vibe.sh`, `vibe_enhanced.sh`, `push.sh`, `dev.sh`

### 📝 Временные файлы редакторов:
- `.#*` - Emacs temporary files
- `\#*\#` - Emacs backup files
- `*.swp`, `*.swo` - Vim swap files
- `*~` - Backup files
- `.benchmarking` - Editor temp files

### 📋 Документация для разработки:
- `COMMIT_INSTRUCTIONS.md`
- `GITHUB_COMMIT.md`
- `EXIF_IMPROVEMENTS.md` (можно оставить, но на всякий случай)
- `VIBE_CODING.md`
- `PERMANENT_SOLUTION.md`
- `PATH_FIX.md`

### 🏰 MemPalace артефакты:
- `mempalace.yaml`
- `entities.json`
- `.last_size`
- `compilation_errors.txt`

## 🎯 Что попадёт в репозиторий:
- Исходный код (src/)
- Frontend файлы (frontend/)
- Документация (README.md, CLAUDE.md)
- Конфигурация (Cargo.toml)
- Тесты (tests/)

## 🚀 Следующий шаг:
```bash
cd /Users/dmitriiromanov/claude_code/photomap
git add .gitignore
git commit -m "Update .gitignore: exclude dev scripts and temp files"
git push origin main
```
