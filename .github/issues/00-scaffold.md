# 0. Каркас: workspace, Tauri, CI

## Цель
Создать основу монорепозитория, чтобы приложение запускалось на macOS и Android, а CI проверял код.

## Задачи
- [ ] `Cargo.toml` workspace: `crates/tuner-core`, `crates/tuner-audio`, `crates/tuner-storage`, `src-tauri`; edition 2021, `resolver = "2"`, release-профиль (`lto`, `codegen-units = 1`, `panic = "abort"`, `strip`).
- [ ] `ui/`: Vite + Svelte 5 + TypeScript.
- [ ] `npm run tauri init`: `productName = Unisono`, `identifier = app.unisono.tuner`, окно 900×640.
- [ ] `npm run tauri android init`; `gen/android` коммитится.
- [ ] CI `.github/workflows/ci.yml` уже добавлен: джобы включатся, когда появятся `Cargo.toml` и `ui/package.json`.
- [ ] Пустые приложения запускаются на macOS и Android; `cargo test` зелёный.

## Готово, когда
`cargo test`, `cargo clippy -D warnings`, `npm --prefix ui run build` и `make -C docs html` проходят; пустое окно открывается на macOS и Android.

## Ссылки
`HANDOFF.md` §5, `docs/source/03_architecture.md`, `docs/source/09_plan.md`.
