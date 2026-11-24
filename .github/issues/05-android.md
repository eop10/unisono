# 5. Android: oboe, Kotlin-шим, APK

## Цель
Захват звука на Android и сборка APK с собственным ключом.

## Задачи
- [x] `OboeSource`: `AudioFormat::Float`, `InputPreset::VoiceRecognition`, 48 кГц, `rtrb`.
- [x] Kotlin-плагин: `requestMic`, `keepScreenOn`, `vibrate`; регистрация в `MainActivity`.
- [x] Манифест: `RECORD_AUDIO`, `VIBRATE`, без `INTERNET`; lifecycle `onStart/onPause`.
- [x] Настройка `signingConfigs.release` из env.
- [x] Sideload-инструкция в README (Android 8.0+, arm64-v8a/armeabi-v7a, обновление тем же ключом).
- [ ] Ручные тесты на устройствах (Android 8 и 14) и проверка подписи релизного APK (`apksigner verify`).

## Готово, когда
Настройка на двух устройствах; разрешение/отказ обрабатываются; APK ставится и обновляется поверх с тем же ключом.

## Ссылки
`docs/source/07_platform.md` §7.1, `HANDOFF.md` §8.
