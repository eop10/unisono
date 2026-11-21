#!/usr/bin/env bash
# Сборка архива со статическим сайтом — чтобы отправить документацию
# почтой или в мессенджере, без репозитория и хостинга.
#
#   ./tools/bundle.sh
#   -> dist/documentation-html-YYYY-MM-DD.zip

set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"

STAMP="$(date +%F)"
OUT="$ROOT/dist/documentation-html-$STAMP.zip"

./.venv/bin/sphinx-build -b html source build/html >/dev/null
mkdir -p "$ROOT/dist"
rm -f "$OUT"

# Каталог внутри архива, чтобы после распаковки всё лежало в одной папке.
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp -R build/html "$STAGE/documentation"

(cd "$STAGE" && zip -qr -X "$OUT" documentation)

echo "  Готово: dist/$(basename "$OUT") ($(du -h "$OUT" | cut -f1))"
echo "  Внутри: documentation/index.html — открывается двойным кликом."
