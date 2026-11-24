#!/usr/bin/env bash
# Per-arch DMG для macOS (docs/source/07_platform.md §7.2.2–7.2.3, 08_build_test.md §8.6).
# Собирает app+dmg отдельно для aarch64 и x86_64, проверяет оба бандла и пишет SHA-256.
# Падает на первой непрошедшей проверке (set -euo pipefail).
#
# Universal не используем (rev 4.1, приёмка 4B): macOS/TCC не сохраняет разрешение
# микрофона для universal ad-hoc бинарников — системный запрос при каждом запуске.
# На одинарной архитектуре грант персистится.

set -euo pipefail

# В headless/скриптовом окружении Finder AppleScript виснет (-1712); CI=true
# включает --skip-jenkins в bundle_dmg.sh.
export CI="${CI:-true}"

cd "$(dirname "$0")/.."
ROOT="$(pwd)"

echo "==> Зависимости"
npm ci
npm --prefix ui ci

echo "==> Rust-таргеты per-arch"
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# Проверки одного бандла: $1 — .app, $2 — .dmg.
check_bundle() {
  # Разрешение микрофона обязано быть в Info.plist (7.2.2).
  # Без -q: grep читает весь вывод, иначе plutil/codesign ловят SIGPIPE (pipefail).
  plutil -p "$1/Contents/Info.plist" | grep NSMicrophoneUsageDescription >/dev/null
  plutil -p "$1/Contents/Info.plist" | grep '"LSMinimumSystemVersion" => "12.0"' >/dev/null
  # Целостность подписи и Info.plist entries (7.2.3, ревью 3.6).
  codesign --verify --deep --strict "$1"
  codesign -dv "$1" 2>&1 | grep "Info.plist entries=" >/dev/null
  codesign -dv "$1" 2>&1 | grep adhoc >/dev/null
  hdiutil verify "$2"
}

declare -a BUILT=()

for TARGET in aarch64-apple-darwin x86_64-apple-darwin; do
  echo "==> Сборка $TARGET (app + dmg)"
  npm run tauri build -- --target "$TARGET"

  BUNDLE="$ROOT/target/$TARGET/release/bundle"
  APP="$BUNDLE/macos/Unisono.app"
  shopt -s nullglob
  dmgs=( "$BUNDLE/dmg"/Unisono_*.dmg )
  if (( ${#dmgs[@]} != 1 )); then
    echo "ожидался один DMG для $TARGET, найдено: ${#dmgs[@]}" >&2
    exit 1
  fi
  DMG="${dmgs[0]}"

  echo "==> Проверки бандла $TARGET"
  check_bundle "$APP" "$DMG"
  BUILT+=( "$DMG" )
done

echo "==> SHA-256"
mkdir -p "$ROOT/dist"
shasum -a 256 "${BUILT[@]}" | tee "$ROOT/dist/SHA256SUMS"

echo "==> Готово:"
printf '  %s\n' "${BUILT[@]}"
