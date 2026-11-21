# 5. Android: oboe, Kotlin-шим, APK

## Цель
Захват звука на Android и сборка APK с собственным ключом.

## Задачи
- [ ] `OboeSource`: `AudioFormat::Float`, `InputPreset::VoiceRecognition`, 48 кГц, `rtrb`.
- [ ] Kotlin-плагин: `requestMic`, `keepScreenOn`, `vibrate`; регистрация в `MainActivity`.
- [ ] Манифест: `RECORD_AUDIO`, `VIBRATE`, без `INTERNET`; lifecycle `onStart/onPause`.
- [ ] Настройка `signingConfigs.release` из env; `apksigner verify`.
- [ ] Тесты на Android 8 и 14; sideload-инструкция в README.

## Готово, когда
Настройка на двух устройствах; разрешение/отказ обрабатываются; APK ставится и обновляется поверх с тем же ключом.

## Ссылки
`docs/source/07_platform.md` §7.1, `HANDOFF.md` §8.
