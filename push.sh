#!/bin/bash
# Отправка изменений на GitHub

PROJECT_DIR="/Users/dmitriiromanov/claude_code/photomap"
cd "$PROJECT_DIR"

echo "📤 Отправка на GitHub..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Проверить статус
git status --short

echo ""
echo "🔄 Добавить все изменения? (y/n)"
read -r answer
if [ "$answer" != "y" ]; then
    echo "❌ Отменено"
    exit 0
fi

# Добавить изменения
git add .

# Запросить сообщение коммита
echo ""
echo "📝 Введите сообщение коммита (или Enter для стандартного):"
read -r message

if [ -z "$message" ]; then
    message="Update PhotoMap project"
fi

# Закоммитить
git commit -m "$message"

# Отправить
echo ""
echo "🚀 Отправка на GitHub..."
git push origin main

echo ""
echo "✅ Готово!"
