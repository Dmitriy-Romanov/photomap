#!/bin/bash
# "Vibe coding" скрипт для PhotoMap проекта
# Запускает компиляцию и если есть ошибки - показывает их для исправления

set -e  # Остановиться при ошибке

PROJECT_DIR="/Users/dmitriiromanov/claude_code/photomap"
cd "$PROJECT_DIR"

echo "🔨 Компиляция PhotoMap..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Компилируем
if cargo build --release 2>&1; then
    echo ""
    echo "✅ Компиляция успешна!"

    # Показать размер бинарника
    SIZE=$(stat -f%z target/release/photomap_processor 2>/dev/null || stat -c%s target/release/photomap_processor 2>/dev/null)
    echo "📦 Размер: $SIZE bytes"

    # Предложить запустить
    echo ""
    echo "🚀 Запустить? (y/n)"
    read -r answer
    if [ "$answer" = "y" ]; then
        ./target/release/photomap_processor
    fi
else
    echo ""
    echo "❌ Компиляция не удалась!"
    echo "📋 Скопируйте ошибки выше в Claude Code для исправления"
    exit 1
fi
