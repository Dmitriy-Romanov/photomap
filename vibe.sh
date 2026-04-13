#!/bin/bash
# 🎵 VIBE CODING - Полный цикл разработки
# Изменения → Компиляция → Тест → Push

PROJECT_DIR="/Users/dmitriiromanov/claude_code/photomap"
cd "$PROJECT_DIR"

echo "🎵 VIBE CODING MODE"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Шаг 1: Компиляция
echo "1️⃣  Компиляция..."
if ! cargo build --release 2>&1; then
    echo ""
    echo "❌ Компиляция не удалась!"
    echo "📋 Скопируйте ошибки в Claude Code для исправления"
    exit 1
fi

echo "✅ Компиляция успешна!"

# Шаг 2: Проверка размера
SIZE=$(stat -f%z target/release/photomap_processor 2>/dev/null || stat -c%s target/release/photomap_processor 2>/dev/null)
SIZE_MB=$(echo "scale=2; $SIZE/1024/1024" | bc)
echo "📦 Размер: $SIZE_MB MB"

# Шаг 3: Git статус
echo ""
echo "2️⃣  Git статус:"
git status --short

# Шаг 4: Вопрос о коммите
echo ""
echo "3️⃣  Закоммитить и отправить? (y/n)"
read -r answer

if [ "$answer" = "y" ]; then
    # Добавить изменения
    git add .

    # Сообщение коммита
    echo ""
    echo "📝 Введите сообщение (или Enter для стандартного):"
    read -r message

    if [ -z "$message" ]; then
        message="Update PhotoMap - build $SIZE_MB MB"
    fi

    # Закоммитить
    git commit -m "$message"

    # Отправить
    echo ""
    echo "📤 Отправка на GitHub..."
    git push origin main

    echo ""
    echo "✅ Готово! Размер: $SIZE_MB MB"
else
    echo ""
    echo "⏸️  Изменения не отправлены"
    echo "💾 Бинарник готов: target/release/photomap_processor"
fi
