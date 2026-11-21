#!/usr/bin/env bash
# Показ документации команде по локальной сети — без репозитория и хостинга.
#
# Запускает статический сервер на всех интерфейсах и печатает адрес, который
# можно отправить в чат. Коллеги в той же сети открывают его в браузере.
#
#   ./tools/serve.sh            # порт 8010
#   PORT=9000 ./tools/serve.sh

set -euo pipefail

cd "$(dirname "$0")/.."

PORT="${PORT:-8010}"
BUILD_DIR="build/html"

if [ ! -f "$BUILD_DIR/index.html" ]; then
    echo "Сборки нет — собираю…"
    ./.venv/bin/sphinx-build -b html source "$BUILD_DIR" >/dev/null
fi

# Адрес в локальной сети: первый не-loopback IPv4.
LAN_IP="$(ipconfig getifaddr en0 2>/dev/null || ipconfig getifaddr en1 2>/dev/null || true)"

echo
echo "  Локально:   http://localhost:$PORT/"
if [ -n "$LAN_IP" ]; then
    echo "  Для команды: http://$LAN_IP:$PORT/"
    echo
    echo "  Отправьте вторую ссылку в чат — она работает с телефона и с любого"
    echo "  компьютера в той же сети. Остановить: Ctrl+C."
else
    echo "  Не удалось определить адрес в локальной сети (нет Wi-Fi/Ethernet?)."
fi
echo

exec python3 -m http.server "$PORT" --bind 0.0.0.0 --directory "$BUILD_DIR"
