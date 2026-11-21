#!/usr/bin/env bash
# Рендер Mermaid-диаграмм: source/diagrams/src/*.mmd -> source/diagrams/*.png
#
#   ./tools/render-diagrams.sh
#
# Требуется Node.js (npx) и Chrome. Путь к Chrome можно переопределить:
#   PUPPETEER_EXECUTABLE_PATH=/path/to/chrome ./tools/render-diagrams.sh

set -euo pipefail
cd "$(dirname "$0")/.."

SRC="source/diagrams/src"
OUT="source/diagrams"
CHROME="${PUPPETEER_EXECUTABLE_PATH:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
WIDTH="${DIAGRAM_WIDTH:-1600}"
PUPPETEER_CFG=".puppeteer.json"

if [ ! -x "$CHROME" ]; then
    echo "Ошибка: Chrome не найден: $CHROME" >&2
    echo "Укажите путь через PUPPETEER_EXECUTABLE_PATH." >&2
    exit 1
fi

mkdir -p "$OUT"
cat > "$PUPPETEER_CFG" <<EOF
{"executablePath": "$CHROME", "args": ["--no-sandbox"]}
EOF
trap 'rm -f "$PUPPETEER_CFG"' EXIT

shopt -s nullglob
files=("$SRC"/*.mmd)
if [ ${#files[@]} -eq 0 ]; then
    echo "Нет файлов .mmd в $SRC" >&2
    exit 1
fi

for src in "${files[@]}"; do
    name="$(basename "$src" .mmd)"
    echo "→ $name.png"
    PUPPETEER_SKIP_DOWNLOAD=1 npx -y @mermaid-js/mermaid-cli@11 \
        -i "$src" -o "$OUT/$name.png" \
        -b white -w "$WIDTH" -p "$PUPPETEER_CFG" >/dev/null
done

echo "Готово: $(ls -1 "$OUT"/*.png | wc -l | tr -d ' ') файлов в $OUT"
