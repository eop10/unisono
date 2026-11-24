# 4. macOS: cpal, окно, DMG

## Цель
Подключить реальный микрофон на macOS и собрать per-arch DMG (aarch64, x86_64) без нотаризации.

## Задачи
- [ ] `tuner-audio`: trait `AudioSource`, `CpalSource` (CoreAudio, `f32`, буфер 512), `rtrb`.
- [ ] Перезапуск потока при смене устройства; ошибки → `NoPermission`/`Error`.
- [ ] `src-tauri`: команды `start_listening/stop_listening`, `Channel<TunerState>`, lifecycle окна.
- [ ] `tuner-storage`: `settings.json`, `presets.json`, атомарная запись.
- [ ] Меню Tauri, горячие клавиши, `Info.plist` с `NSMicrophoneUsageDescription`.
- [ ] Per-arch DMG (aarch64, x86_64) + ad-hoc подпись + блок «Первый запуск» в README.

## Готово, когда
Гитара и скрипка настраиваются на Mac; DMG открывается после «Всё равно открыть»; настройки переживают перезапуск.

## Ссылки
`docs/source/07_platform.md` §7.2, `docs/source/08_build_test.md` §8.6.
