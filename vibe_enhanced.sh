#!/bin/bash
# 🎵 VIBE CODING v2.0 - ПЕРМАНЕНТНОЕ решение
# Учитывая что sandbox - это новая норма

PROJECT_DIR="/Users/dmitriiromanov/claude_code/photomap"
cd "$PROJECT_DIR"

echo "🎵 VIBE CODING v2.0 (Permanent Edition)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Цвета для вывода
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Лог ошибок
ERROR_LOG="compilation_errors.txt"

# Шаг 1: Компиляция с сохранением ошибок
echo -e "${YELLOW}1️⃣  Компиляция...${NC}"
if cargo build --release 2> "$ERROR_LOG"; then
    echo -e "${GREEN}✅ Компиляция успешна!${NC}"
    rm -f "$ERROR_LOG" # Очищаем лог при успехе

    # Шаг 2: Анализ размера
    SIZE=$(stat -f%z target/release/photomap_processor 2>/dev/null || stat -c%s target/release/photomap_processor 2>/dev/null)
    SIZE_MB=$(echo "scale=2; $SIZE/1024/1024" | bc)

    echo -e "${GREEN}📦 Размер: $SIZE_MB MB${NC}"

    # Шаг 3: Сравнение с предыдущим
    if [ -f ".last_size" ]; then
        LAST_SIZE=$(cat .last_size)
        DIFF=$(echo "scale=2; $SIZE_MB - $LAST_SIZE" | bc)
        if [ $(echo "$DIFF < 0" | bc) -eq 1 ]; then
            echo -e "${GREEN}📉 Размер уменьшился на: ${DIFF} MB${NC}"
        elif [ $(echo "$DIFF > 0" | bc) -eq 1 ]; then
            echo -e "${YELLOW}📈 Размер увеличился на: +${DIFF} MB${NC}"
        else
            echo -e "${GREEN}➡️  Размер не изменился${NC}"
        fi
    fi
    echo "$SIZE_MB" > .last_size

    # Шаг 4: Git статус
    echo ""
    echo -e "${YELLOW}2️⃣  Git статус:${NC}"
    git status --short

    # Шаг 5: Интерактивный коммит
    echo ""
    echo -e "${YELLOW}3️⃣  Закоммитить изменения?${NC}"
    echo "   [1] Да + push на GitHub"
    echo "   [2] Только коммит (без push)"
    echo "   [3] Только протестировать"
    echo "   [4] Отмена"
    read -p "Выбор: " choice

    case $choice in
        1)
            # Полный цикл с push
            echo -e "${YELLOW}📝 Введите сообщение коммита:${NC}"
            read -r message

            if [ -z "$message" ]; then
                message="Update PhotoMap - build $SIZE_MB MB"
            fi

            git add .
            git commit -m "$message"
            echo -e "${GREEN}📤 Отправка на GitHub...${NC}"
            git push origin main

            echo ""
            echo -e "${GREEN}✅ Готово! Размер: $SIZE_MB MB${NC}"
            echo -e "${GREEN}🔗 GitHub обновлён${NC}"
            ;;
        2)
            # Только коммит
            echo -e "${YELLOW}📝 Введите сообщение коммита:${NC}"
            read -r message

            if [ -z "$message" ]; then
                message="Update PhotoMap - build $SIZE_MB MB"
            fi

            git add .
            git commit -m "$message"

            echo ""
            echo -e "${GREEN}✅ Закоммичено! Размер: $SIZE_MB MB${NC}"
            echo -e "${YELLOW}⚠️  Не отправлено на GitHub (сделайте: git push)${NC}"
            ;;
        3)
            # Только тест
            echo ""
            echo -e "${GREEN}✅ Бинарник готов: target/release/photomap_processor${NC}"
            echo -e "${YELLOW}💡 Запустить? (y/n)${NC}"
            read -r run_answer
            if [ "$run_answer" = "y" ]; then
                ./target/release/photomap_processor
            fi
            ;;
        4)
            echo -e "${YELLOW}⏸️  Отменено${NC}"
            ;;
        *)
            echo -e "${RED}❌ Неверный выбор${NC}"
            exit 1
            ;;
    esac

else
    # Ошибка компиляции
    echo -e "${RED}❌ Компиляция не удалась!${NC}"
    echo ""
    echo -e "${YELLOW}📋 Последние 20 строк ошибок:${NC}"
    tail -20 "$ERROR_LOG"

    echo ""
    echo -e "${YELLOW}💡 Скопируйте ошибки выше в Claude Code${NC}"
    echo -e "${YELLOW}💡 Полный лог сохранён в: $ERROR_LOG${NC}"

    # Предлагаем автоматическое копирование
    echo ""
    echo -e "${YELLOW}📋 Копировать ошибки в буфер обмена? (y/n)${NC}"
    read -r copy_answer
    if [ "$copy_answer" = "y" ]; then
        cat "$ERROR_LOG" | pbcopy
        echo -e "${GREEN}✅ Ошибки скопированы! Вставьте в Claude Cmd+V${NC}"
    fi

    exit 1
fi
