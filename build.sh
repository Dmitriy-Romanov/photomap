#!/bin/bash
# Быстрая компиляция с автоматическим показом ошибок

PROJECT_DIR="/Users/dmitriiromanov/claude_code/photomap"
cd "$PROJECT_DIR"

echo "🔨 Компиляция..."

# Компилируем и сохраняем вывод
OUTPUT=$(cargo build --release 2>&1)

if [ $? -eq 0 ]; then
    echo "✅ Успех!"
    SIZE=$(stat -f%z target/release/photomap_processor 2>/dev/null || stat -c%s target/release/photomap_processor 2>/dev/null)
    echo "📦 Размер: $(echo "scale=2; $SIZE/1024/1024" | bc) MB"
else
    echo "❌ Ошибки компиляции:"
    echo "$OUTPUT"
    echo ""
    echo "📋 Скопируйте ошибки выше в Claude Code"
fi
