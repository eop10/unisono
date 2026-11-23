#!/usr/bin/env bash
# Universal DMG для macOS (docs/source/07_platform.md §7.2.2–7.2.3, §8.6).
# Собирает app+dmg для aarch64 и x86_64, проверяет бандл и пишет SHA-256.
# Падает на первой непрошедшей проверке (set -euo pipefail).

set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"

echo "==> Зависимости"
npm ci
npm --prefix ui ci

echo "==> Rust-таргеты Universal"
rustup target add aarch64-apple-darwin x86_64-apple-darwin

echo "==> Сборка universal-apple-darwin (app + dmg)"
npm run tauri build -- --target universal-apple-darwin

APP="$ROOT/target/universal-apple-darwin/release/bundle/macos/Unisono.app"
DMG_DIR="$ROOT/target/universal-apple-darwin/release/bundle/dmg"
shopt -s nullglob
dmgs=( "$DMG_DIR"/Unisono_*_universal.dmg )
if (( ${#dmgs[@]} != 1 )); then
  echo "ожидался один Universal DMG, найдено: ${#dmgs[@]}" >&2
  exit 1
fi
DMG="${dmgs[0]}"

echo "==> Проверки бандла"
# Разрешение микрофона обязано быть в Info.plist (7.2.2).
# Без -q: grep читает весь вывод, иначе plutil/codesign ловят SIGPIPE (pipefail).
plutil -p "$APP/Contents/Info.plist" | grep NSMicrophoneUsageDescription >/dev/null
plutil -p "$APP/Contents/Info.plist" | grep '"LSMinimumSystemVersion" => "12.0"' >/dev/null
# Проверка целостности подписи и Info.plist entries (ревью 3.6, 7.2.3).
codesign --verify --deep --strict "$APP"
codesign -dv "$APP" 2>&1 | grep "Info.plist entries=" >/dev/null
codesign -dv "$APP" 2>&1 | grep adhoc >/dev/null
hdiutil verify "$DMG"

echo "==> SHA-256"
mkdir -p "$ROOT/dist"
shasum -a 256 "$DMG" | tee "$ROOT/dist/SHA256SUMS"

echo "==> Готово: $DMG"
