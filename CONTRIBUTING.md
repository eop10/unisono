# Вклад в Unisono / Contributing

Спасибо за интерес к **Unisono**!

## Как предложить изменение

1. Сделайте форк репозитория и создайте ветку от `main`.
2. Перед отправкой PR убедитесь, что проходят все проверки:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   npm --prefix ui run check
   npm --prefix ui run test
   npm --prefix ui run build
   ```
3. Откройте Pull Request с кратким описанием изменений.

## Архитектурные границы

- Логика DSP, детектирования высоты тона, строёв и аудиозахвата живёт в Rust-крейтах (`crates/tuner-core`, `crates/tuner-audio`, `crates/tuner-storage`).
- UI (`ui/`) не содержит DSP-логики и общается с ядром через команды Tauri (`src-tauri/src/commands.rs`).
- Приложение остаётся полностью офлайн: без сетевых запросов и телеметрии.

## Лицензия

Код распространяется под [Apache License 2.0](LICENSE). Отправляя PR, вы соглашаетесь на публикацию вклада под этой же лицензией. Название **Unisono** и ассеты в `brand/` не входят в Apache 2.0.
