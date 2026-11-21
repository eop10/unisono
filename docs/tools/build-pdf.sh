#!/usr/bin/env bash
# Сборка PDF без LaTeX: Sphinx singlehtml + печать в PDF из headless Chrome.
#
#   ./tools/build-pdf.sh
#   -> dist/documentation.pdf  (A4 landscape)
#
# Почему так: latex на машинах команды не установлен, а Chrome уже есть.
# Стили печати лежат в source/_static/custom.css, блок @media print.

set -euo pipefail

cd "$(dirname "$0")/.."

CHROME="${PUPPETEER_EXECUTABLE_PATH:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
OUT_DIR="dist"
OUT_PDF="$OUT_DIR/documentation.pdf"

if [ ! -x "$CHROME" ]; then
    echo "Ошибка: Chrome не найден: $CHROME" >&2
    echo "Укажите путь через PUPPETEER_EXECUTABLE_PATH." >&2
    exit 1
fi

echo "→ сборка singlehtml"
./.venv/bin/sphinx-build -b singlehtml source build/singlehtml >/dev/null

# Chrome печатает только по http(s): file:// с локальными ресурсами
# в headless-режиме отдаёт пустой PDF.
PORT="${PDF_PORT:-8020}"
python3 -m http.server "$PORT" --bind 127.0.0.1 --directory build/singlehtml >/dev/null 2>&1 &
SERVER_PID=$!
# Отсоединяем от job control, иначе при kill оболочка печатает «Terminated».
disown "$SERVER_PID" 2>/dev/null || true
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT

for _ in $(seq 1 40); do
    if curl -sf -o /dev/null "http://127.0.0.1:$PORT/index.html"; then break; fi
    sleep 0.25
done

mkdir -p "$OUT_DIR"

echo "→ печать в PDF"
# virtual-time-budget даёт MathJax время отрисовать формулы до снимка.
"$CHROME" --headless=new --disable-gpu \
    --no-pdf-header-footer \
    --virtual-time-budget=30000 \
    --print-to-pdf="$OUT_PDF" \
    "http://127.0.0.1:$PORT/index.html" >/dev/null 2>&1

kill "$SERVER_PID" 2>/dev/null || true
trap - EXIT

SIZE_BYTES="$(stat -f%z "$OUT_PDF")"
if [ "$SIZE_BYTES" -lt 100000 ]; then
    echo "Ошибка: PDF подозрительно мал ($SIZE_BYTES байт) — вероятно, страница не загрузилась." >&2
    exit 1
fi

echo
echo "  Готово: $OUT_PDF ($(du -h "$OUT_PDF" | cut -f1))"
echo
echo "  Проверка перед отправкой: откройте файл и убедитесь, что на месте"
echo "  диаграммы и формулы на месте."
