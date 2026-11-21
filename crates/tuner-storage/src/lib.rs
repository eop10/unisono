//! tuner-storage: `settings.json` и `presets.json` (docs/source/05_data.md).
//!
//! `serde` + `serde_json`, `schema_version`, атомарная запись
//! (tmp + `sync_all` + `rename`). Библиотека принимает `&Path`;
//! пути платформы даёт `tauri::path()` в приложении.
