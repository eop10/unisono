//! tuner-storage: `settings.json` и `presets.json` (docs/source/05_data.md).
//!
//! `serde` + `serde_json`, `schema_version` в обоих файлах (§5.3, 5.6),
//! атомарная запись `*.tmp` → `sync_all()` → `rename` (§5.2). Библиотека
//! принимает `&Path`; пути платформы даёт `tauri::path()` в приложении.
//! Аудио не хранится, телеметрии нет (5.1, ADR-003).
//!
//! Правило чтения (§5.3): значение вне диапазона заменяется значением по
//! умолчанию — `Settings::sanitize()`; битый JSON уходит в карантин
//! `<имя>.corrupt-<unix_ts>.json`, приложение стартует с дефолтов (§5.8).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tuner_core::music::{InstrumentRegistry, INSTRUMENTS};

/// Текущая версия схемы; пишется в оба файла (§5.3).
pub const SCHEMA_VERSION: u32 = 3;

/// Лимит пользовательских строёв на инструмент (§5.7).
pub const MAX_PRESETS_PER_INSTRUMENT: usize = 20;

/// Границы опорной частоты и шаг (§5.3): 400,0–480,0 с шагом 0,1.
const REFERENCE_MIN: f64 = 400.0;
const REFERENCE_MAX: f64 = 480.0;

/// MIDI-диапазон струн пресета (§5.6): 21..96, 1..8 элементов.
const MIDI_MIN: u8 = 21;
const MIDI_MAX: u8 = 96;
const STRINGS_MAX: usize = 8;

/// Префикс ID пользовательских строёв (§5.6): `custom.<uuid>` —
/// sanitize сохраняет такой `tuning_id`, даже если это не встроенный строй.
const CUSTOM_ID_PREFIX: &str = "custom.";

/// Нотация (§5.3). `Unknown` — нераспознанное значение из файла,
/// заменяется дефолтом в `sanitize` (не считаем файл битым).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum Notation {
    #[default]
    Latin,
    Solfeggio,
    #[serde(other)]
    Unknown,
}

/// Тема (§5.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
    #[serde(other)]
    Unknown,
}

/// Язык интерфейса (§5.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum Language {
    #[default]
    Ru,
    En,
    #[serde(other)]
    Unknown,
}

/// Чувствительность (§5.3): порог clarity 0,75 / 0,60 / 0,45.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum Sensitivity {
    Low,
    #[default]
    Normal,
    High,
    #[serde(other)]
    Unknown,
}

/// Настройки (§5.3). Полные `snake_case`-имена — это формат файла.
/// `#[serde(default)]` обязателен: при добавлении поля старый файл читается.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub schema_version: u32,
    /// Опорная частота A4, Гц: 400,0–480,0 с шагом 0,1.
    pub reference_hz: f64,
    /// ID инструмента из 5.5 или `chromatic`.
    pub instrument_id: String,
    /// Строй выбранного инструмента; `None` для хроматики.
    pub tuning_id: Option<String>,
    pub notation: Notation,
    pub theme: Theme,
    pub language: Language,
    pub haptics: bool,
    pub sound: bool,
    /// Допуск «в строе»: 2, 3 или 5.
    pub tolerance_cents: u8,
    /// Порог входного RMS-гейта, dBFS: -55, -45 или -35.
    pub gate_db: i8,
    pub sensitivity: Sensitivity,
    /// Разрешает окно 16384 в хроматическом режиме.
    pub low_range: bool,
    pub big_type: bool,
    pub strobe: bool,
    pub onboarding_done: bool,
    pub input_device: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            reference_hz: 440.0,
            instrument_id: "guitar".into(),
            tuning_id: Some("standard_e".into()),
            notation: Notation::Latin,
            theme: Theme::System,
            language: Language::Ru,
            haptics: true,
            sound: false,
            tolerance_cents: 3,
            gate_db: -45,
            sensitivity: Sensitivity::Normal,
            low_range: false,
            big_type: false,
            strobe: false,
            onboarding_done: false,
            input_device: None,
        }
    }
}

impl Settings {
    /// Значения вне диапазона → дефолтные, без паники (§5.3).
    /// `schema_version` не трогает — его выставляет миграция при чтении.
    pub fn sanitize(mut self) -> Self {
        // A4: границы 400–480 и шаг 0,1.
        let hz = self.reference_hz.clamp(REFERENCE_MIN, REFERENCE_MAX);
        self.reference_hz = (hz * 10.0).round() / 10.0;

        // Допуск: только 2 / 3 / 5.
        if !matches!(self.tolerance_cents, 2 | 3 | 5) {
            self.tolerance_cents = 3;
        }
        if !matches!(self.gate_db, -55 | -45 | -35) {
            self.gate_db = -45;
        }

        // Enum-значения из файла, которые мы не знаем.
        if self.notation == Notation::Unknown {
            self.notation = Notation::Latin;
        }
        if self.theme == Theme::Unknown {
            self.theme = Theme::System;
        }
        if self.language == Language::Unknown {
            self.language = Language::Ru;
        }
        if self.sensitivity == Sensitivity::Unknown {
            self.sensitivity = Sensitivity::Normal;
        }

        // Инструмент: известный из реестра или хроматика, иначе гитара.
        let known = |id: &str| id == "chromatic" || INSTRUMENTS.iter().any(|i| i.id == id);
        if !known(&self.instrument_id) {
            self.instrument_id = "guitar".into();
        }
        if self.instrument_id == "chromatic" {
            // У хроматики строёв нет (D-6).
            self.tuning_id = None;
        } else {
            let registry = InstrumentRegistry::new();
            let builtin = |id: &str| registry.tuning(&self.instrument_id, id).is_some();
            match self.tuning_id.as_deref() {
                // `custom.*` сохраняем: это пользовательский строй из presets.json,
                // проверка произойдёт в UI после загрузки пресетов (part 2).
                Some(id) if id.starts_with(CUSTOM_ID_PREFIX) => {}
                Some(id) if builtin(id) => {}
                // Чужой/несуществующий строй → первый встроенный инструмента.
                _ => {
                    self.tuning_id = registry
                        .tunings_for(&self.instrument_id)
                        .first()
                        .map(|t| t.id.clone());
                }
            }
        }

        // Аудиовход: пустая строка после trim превращается в None.
        self.input_device = self.input_device.and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

        self
    }
}

/// Пользовательский строй (§5.6, TuningPreset): запись `presets.json`.
/// Встроенные строи в файл не пишутся — они в коде.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct PresetRecord {
    /// `custom.<uuid>`.
    pub id: String,
    pub instrument_id: String,
    /// 1..24 символа, пользовательский текст.
    pub name: String,
    /// MIDI 21..96, 1..8 элементов; дубли разрешены (унисон).
    pub strings: Vec<u8>,
    #[serde(default)]
    pub built_in: bool,
}

impl PresetRecord {
    /// Структурная валидность (§5.6, 5.7): струны 1..8 из MIDI 21..96.
    /// Имя проверяет UI при вводе; здесь — защита файла от мусора.
    pub fn is_valid(&self) -> bool {
        (1..=STRINGS_MAX).contains(&self.strings.len())
            && self
                .strings
                .iter()
                .all(|&m| (MIDI_MIN..=MIDI_MAX).contains(&m))
            && !self.name.trim().is_empty()
            && self.name.chars().count() <= 24
            && self.id.starts_with(CUSTOM_ID_PREFIX)
    }
}

/// Обёртка `presets.json` (§5.6).
#[derive(serde::Serialize, serde::Deserialize)]
struct PresetsFile {
    schema_version: u32,
    presets: Vec<PresetRecord>,
}

/// Ошибки хранения: значения, не паники — настройки не критичный путь.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("не удалось создать каталог {path}: {message}")]
    CreateDir { path: String, message: String },
    #[error("ошибка записи {path}: {message}")]
    Write { path: String, message: String },
    #[error("пресет {id} невалиден: {reason}")]
    InvalidPreset { id: String, reason: String },
    #[error("лимит {limit} пользовательских строёв на инструмент {instrument}")]
    PresetLimit { instrument: String, limit: usize },
    #[error("схема файла {found} новее поддерживаемой {current}, перезапись запрещена")]
    NewerSchema { found: u32, current: u32 },
}

/// Хранилище в каталоге конфигурации: `settings.json`, `presets.json` (§5.2).
/// Путь задаёт приложение; крейт про Tauri не знает.
pub struct Store {
    dir: PathBuf,
    settings_read_only: AtomicBool,
    newer_schema_found: AtomicU32,
}

impl Store {
    /// Открывает (и создаёт) каталог хранения.
    pub fn open(dir: &Path) -> Result<Self, StoreError> {
        fs::create_dir_all(dir).map_err(|e| StoreError::CreateDir {
            path: dir.display().to_string(),
            message: e.to_string(),
        })?;
        tracing::info!(dir = %dir.display(), "хранилище открыто");
        Ok(Self {
            dir: dir.to_path_buf(),
            settings_read_only: AtomicBool::new(false),
            newer_schema_found: AtomicU32::new(0),
        })
    }

    fn settings_path(&self) -> PathBuf {
        self.dir.join("settings.json")
    }

    fn presets_path(&self) -> PathBuf {
        self.dir.join("presets.json")
    }

    /// Настройки: нет файла — дефолт; битый — карантин и дефолт;
    /// версия меньше текущей — миграция (§5.8), больше — читаем известные
    /// поля и файл не перезаписываем.
    pub fn load_settings(&self) -> Settings {
        let path = self.settings_path();
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Settings::default(),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "настройки не читаются");
                return Settings::default();
            }
        };
        let value: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                quarantine(&path, &format!("битый JSON: {e}"));
                return Settings::default();
            }
        };
        // §5.8: версия приходит из файла; отсутствие считаем схемой 1.
        let version = value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1) as u32;
        match serde_json::from_value::<Settings>(value) {
            Ok(mut s) => {
                if version > SCHEMA_VERSION {
                    self.settings_read_only.store(true, Ordering::SeqCst);
                    self.newer_schema_found.store(version, Ordering::SeqCst);
                    // Файл от более новой версии: читаем известные поля,
                    // ничего не перезаписываем, уведомляем (§5.8.3).
                    tracing::warn!(
                        path = %path.display(),
                        file_version = version,
                        "файл настроек от более новой версии схемы; известные поля прочитаны"
                    );
                    s.schema_version = version;
                } else {
                    self.settings_read_only.store(false, Ordering::SeqCst);
                    self.newer_schema_found.store(0, Ordering::SeqCst);
                    if version < SCHEMA_VERSION {
                        // Миграция 1→2 (инструмент/строй/язык) и 2→3 (аудиовход):
                        // новые поля приходят дефолтами (`#[serde(default)]`),
                        // версия становится 3.
                        tracing::info!(
                            path = %path.display(),
                            from = version,
                            to = SCHEMA_VERSION,
                            "миграция схемы настроек"
                        );
                    }
                    s.schema_version = SCHEMA_VERSION;
                }
                s.sanitize()
            }
            Err(e) => {
                quarantine(&path, &format!("несовместимая структура: {e}"));
                Settings::default()
            }
        }
    }

    /// Атомарная запись настроек; `schema_version` всегда текущая.
    /// Если файл от более новой версии схемы — возвращает ошибку и не перезаписывает (3.5, 5.8.3).
    pub fn save_settings(&self, settings: &Settings) -> Result<(), StoreError> {
        if self.settings_read_only.load(Ordering::SeqCst) {
            return Err(StoreError::NewerSchema {
                found: self.newer_schema_found.load(Ordering::SeqCst),
                current: SCHEMA_VERSION,
            });
        }
        let mut settings = settings.clone().sanitize();
        settings.schema_version = SCHEMA_VERSION;
        atomic_write_json(&self.settings_path(), &settings)
    }

    /// Пресеты: нет файла — пусто; битый — карантин и пусто. Записи со
    /// струнами вне 21–96, пустыми или неизвестным инструментом
    /// пропускаются с предупреждением, загрузку не роняют (§5.8).
    pub fn load_presets(&self) -> Vec<PresetRecord> {
        let path = self.presets_path();
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "пресеты не читаются");
                return Vec::new();
            }
        };
        let file: PresetsFile = match serde_json::from_str(&raw) {
            Ok(f) => f,
            Err(e) => {
                quarantine(&path, &format!("битый JSON: {e}"));
                return Vec::new();
            }
        };
        if file.schema_version > SCHEMA_VERSION {
            tracing::warn!(
                path = %path.display(),
                file_version = file.schema_version,
                "пресеты от более новой версии схемы; известные поля прочитаны"
            );
        }
        let registry = InstrumentRegistry::new();
        let mut kept = Vec::with_capacity(file.presets.len());
        for preset in file.presets {
            if !registry
                .instruments()
                .iter()
                .any(|i| i.id == preset.instrument_id)
            {
                tracing::warn!(
                    id = %preset.id,
                    instrument = %preset.instrument_id,
                    "пресет с неизвестным инструментом пропущен"
                );
                continue;
            }
            if !preset.is_valid() {
                tracing::warn!(id = %preset.id, "пресет вне диапазона пропущен");
                continue;
            }
            kept.push(preset);
        }
        kept
    }

    /// Атомарная запись пресетов; лимит 20 на инструмент проверяется здесь.
    pub fn save_presets(&self, presets: &[PresetRecord]) -> Result<(), StoreError> {
        let registry = InstrumentRegistry::new();
        for preset in presets {
            if !registry
                .instruments()
                .iter()
                .any(|i| i.id == preset.instrument_id)
            {
                return Err(StoreError::InvalidPreset {
                    id: preset.id.clone(),
                    reason: format!("неизвестный инструмент {}", preset.instrument_id),
                });
            }
            if !preset.is_valid() {
                return Err(StoreError::InvalidPreset {
                    id: preset.id.clone(),
                    reason: "невалидный пресет (имя, ID или струны вне диапазона)".into(),
                });
            }
        }
        let mut per_instrument: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for preset in presets {
            let count = per_instrument
                .entry(preset.instrument_id.as_str())
                .or_default();
            *count += 1;
            if *count > MAX_PRESETS_PER_INSTRUMENT {
                return Err(StoreError::PresetLimit {
                    instrument: preset.instrument_id.clone(),
                    limit: MAX_PRESETS_PER_INSTRUMENT,
                });
            }
        }
        let file = PresetsFile {
            schema_version: SCHEMA_VERSION,
            presets: presets.to_vec(),
        };
        atomic_write_json(&self.presets_path(), &file)
    }
}

/// Атомарная запись (§5.2): `settings.json.tmp` → `sync_all()` → `rename`.
fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), StoreError> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "data.json".into());
    let tmp = path.with_file_name(format!("{name}.tmp"));
    let write = || -> std::io::Result<()> {
        let data = serde_json::to_string_pretty(value)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut file = fs::File::create(&tmp)?;
        std::io::Write::write_all(&mut file, data.as_bytes())?;
        // Сначала сбрасываем на диск, потом атомарно подменяем имя.
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        Ok(())
    };
    write().map_err(|e| StoreError::Write {
        path: path.display().to_string(),
        message: e.to_string(),
    })
}

/// Карантин битого файла: `<имя>.corrupt-<unix_ts>.json` (§5.8.4).
fn quarantine(path: &Path, reason: &str) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "data".into());
    let quarantined = path.with_file_name(format!("{stem}.corrupt-{ts}.json"));
    match fs::rename(path, &quarantined) {
        Ok(()) => tracing::warn!(
            path = %path.display(),
            quarantine = %quarantined.display(),
            reason,
            "битый файл отправлен в карантин"
        ),
        Err(e) => tracing::warn!(
            path = %path.display(),
            reason,
            error = %e,
            "битый файл не удалось переименовать"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(dir.path()).expect("store");
        (dir, store)
    }

    #[test]
    fn settings_round_trip() {
        let (_dir, store) = tmp_store();
        let s = Settings {
            reference_hz: 442.0,
            instrument_id: "violin".into(),
            tuning_id: Some("violin_std".into()),
            language: Language::En,
            theme: Theme::Light,
            tolerance_cents: 5,
            ..Settings::default()
        };
        store.save_settings(&s).expect("save");
        let loaded = store.load_settings();
        assert_eq!(loaded, s.sanitize());
        assert_eq!(loaded.schema_version, SCHEMA_VERSION);
        // snake_case-поля и camelCase-совместимые значения в файле.
        let raw = fs::read_to_string(store.settings_path()).unwrap();
        assert!(raw.contains("\"reference_hz\": 442.0"));
        assert!(raw.contains("\"instrument_id\": \"violin\""));
        assert!(raw.contains("\"schema_version\": 3"));
    }

    #[test]
    fn missing_files_give_defaults() {
        let (_dir, store) = tmp_store();
        assert_eq!(store.load_settings(), Settings::default());
        assert!(store.load_presets().is_empty());
    }

    #[test]
    fn atomic_write_leaves_no_tmp() {
        let (_dir, store) = tmp_store();
        store.save_settings(&Settings::default()).unwrap();
        let entries: Vec<_> = fs::read_dir(&store.dir).unwrap().collect();
        assert_eq!(entries.len(), 1, "только settings.json, без .tmp");
        assert!(store.settings_path().exists());
    }

    #[test]
    fn corrupt_settings_quarantined() {
        let (dir, store) = tmp_store();
        fs::write(store.settings_path(), "{ не json").unwrap();
        let s = store.load_settings();
        assert_eq!(s, Settings::default());
        // Карантин появился, оригинал исчез.
        let mut names: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert!(
            names.iter().any(|n| n.starts_with("settings.corrupt-")),
            "файлы: {names:?}"
        );
        assert!(!names.iter().any(|n| n == "settings.json"));
        // Повторная загрузка — дефолт, карантин не растёт бесконечно.
        assert_eq!(store.load_settings(), Settings::default());
    }

    #[test]
    fn corrupt_presets_quarantined() {
        let (dir, store) = tmp_store();
        fs::write(store.presets_path(), "]]не json[").unwrap();
        assert!(store.load_presets().is_empty());
        let names: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            names.iter().any(|n| n.starts_with("presets.corrupt-")),
            "файлы: {names:?}"
        );
    }

    #[test]
    fn migration_v1_to_v2() {
        let (_dir, store) = tmp_store();
        // Схема 1: без instrument/tuning/language и без schema_version.
        let v1 = r#"{
            "reference_hz": 442.5,
            "notation": "Latin",
            "theme": "Dark",
            "haptics": false,
            "sound": true,
            "tolerance_cents": 5,
            "sensitivity": "High",
            "low_range": true,
            "big_type": true,
            "onboarding_done": true
        }"#;
        fs::write(store.settings_path(), v1).unwrap();
        let s = store.load_settings();
        assert_eq!(s.schema_version, SCHEMA_VERSION);
        assert_eq!(s.instrument_id, "guitar");
        assert_eq!(s.tuning_id.as_deref(), Some("standard_e"));
        assert_eq!(s.language, Language::Ru);
        // Известные поля версии 1 сохранены.
        assert_eq!(s.reference_hz, 442.5);
        assert_eq!(s.theme, Theme::Dark);
        assert!(!s.haptics);
        assert!(s.sound);
        assert_eq!(s.tolerance_cents, 5);
        assert_eq!(s.sensitivity, Sensitivity::High);
        assert!(s.low_range && s.big_type && s.onboarding_done);
        assert_eq!(s.input_device, None);
    }

    #[test]
    fn migration_v2_to_v3() {
        let (_dir, store) = tmp_store();
        // Схема 2: с instrument/tuning/language, но без input_device.
        let v2 = r#"{
            "schema_version": 2,
            "reference_hz": 440.0,
            "instrument_id": "bass",
            "tuning_id": "bass_std",
            "notation": "Latin",
            "theme": "Dark",
            "language": "En",
            "haptics": true,
            "sound": false,
            "tolerance_cents": 3,
            "sensitivity": "Normal",
            "low_range": false,
            "big_type": false,
            "onboarding_done": true
        }"#;
        fs::write(store.settings_path(), v2).unwrap();
        let s = store.load_settings();
        assert_eq!(s.schema_version, 3);
        assert_eq!(s.instrument_id, "bass");
        assert_eq!(s.input_device, None);
    }

    #[test]
    fn schema_v3_with_input_device_persists() {
        let (_dir, store) = tmp_store();
        let v3 = r#"{
            "schema_version": 3,
            "reference_hz": 440.0,
            "instrument_id": "guitar",
            "tuning_id": "standard_e",
            "input_device": "AirPods"
        }"#;
        fs::write(store.settings_path(), v3).unwrap();
        let s = store.load_settings();
        assert_eq!(s.schema_version, 3);
        assert_eq!(s.input_device.as_deref(), Some("AirPods"));

        // Сохранение перезаписывает файл со схемой 3 и сохраняет input_device
        store.save_settings(&s).unwrap();
        let loaded = store.load_settings();
        assert_eq!(loaded.schema_version, 3);
        assert_eq!(loaded.input_device.as_deref(), Some("AirPods"));
    }

    #[test]
    fn schema_v4_read_only_known_fields() {
        let (_dir, store) = tmp_store();
        let v4 = r#"{
            "schema_version": 4,
            "reference_hz": 443.0,
            "instrument_id": "violin",
            "tuning_id": "violin_std",
            "input_device": "USB Mic",
            "some_new_v4_field": 123
        }"#;
        fs::write(store.settings_path(), v4).unwrap();
        let before = fs::read_to_string(store.settings_path()).unwrap();
        let s = store.load_settings();
        assert_eq!(s.schema_version, 4);
        assert_eq!(s.reference_hz, 443.0);
        assert_eq!(s.instrument_id, "violin");
        assert_eq!(s.input_device.as_deref(), Some("USB Mic"));
        assert_eq!(fs::read_to_string(store.settings_path()).unwrap(), before);

        let save_res = store.save_settings(&Settings::default());
        assert!(matches!(
            save_res,
            Err(StoreError::NewerSchema {
                found: 4,
                current: SCHEMA_VERSION
            })
        ));
        assert_eq!(fs::read_to_string(store.settings_path()).unwrap(), before);
    }

    #[test]
    fn future_schema_read_known_fields_without_rewrite() {
        let (_dir, store) = tmp_store();
        let v3 = r#"{
            "schema_version": 99,
            "reference_hz": 415.0,
            "instrument_id": "cello",
            "tuning_id": "cello_std",
            "unknown_future_field": {"x": 1}
        }"#;
        fs::write(store.settings_path(), v3).unwrap();
        let before = fs::read_to_string(store.settings_path()).unwrap();
        let s = store.load_settings();
        assert_eq!(s.reference_hz, 415.0);
        assert_eq!(s.instrument_id, "cello");
        // Файл не перезаписан (§5.8.3).
        assert_eq!(fs::read_to_string(store.settings_path()).unwrap(), before);

        // Попытка сохранить при более новой версии схемы возвращает Err и не пишет файл (3.5)
        let save_res = store.save_settings(&Settings::default());
        assert!(matches!(
            save_res,
            Err(StoreError::NewerSchema {
                found: 99,
                current: SCHEMA_VERSION
            })
        ));
        assert_eq!(fs::read_to_string(store.settings_path()).unwrap(), before);

        // Обычный файл (схема <= SCHEMA_VERSION) сохраняется как раньше
        let (_dir2, store2) = tmp_store();
        assert!(store2.save_settings(&Settings::default()).is_ok());
        assert!(store2.settings_path().exists());
    }

    #[test]
    fn sanitize_each_field() {
        // A4: границы и шаг 0,1.
        let s = Settings {
            reference_hz: 999.7,
            ..Settings::default()
        }
        .sanitize();
        assert_eq!(s.reference_hz, 480.0);
        let s = Settings {
            reference_hz: 399.94,
            ..Settings::default()
        }
        .sanitize();
        assert_eq!(s.reference_hz, 400.0);
        let s = Settings {
            reference_hz: 441.26,
            ..Settings::default()
        }
        .sanitize();
        assert_eq!(s.reference_hz, 441.3);

        // Допуск: только 2/3/5.
        assert_eq!(
            Settings {
                tolerance_cents: 4,
                ..Settings::default()
            }
            .sanitize()
            .tolerance_cents,
            3
        );

        // Неизвестный инструмент → гитара.
        let s = Settings {
            instrument_id: "zither".into(),
            tuning_id: Some("violin_std".into()),
            ..Settings::default()
        }
        .sanitize();
        assert_eq!(s.instrument_id, "guitar");
        assert_eq!(s.tuning_id.as_deref(), Some("standard_e"));

        // Чужой строй инструмента → первый встроенный.
        let s = Settings {
            instrument_id: "violin".into(),
            tuning_id: Some("cello_std".into()),
            ..Settings::default()
        }
        .sanitize();
        assert_eq!(s.tuning_id.as_deref(), Some("violin_std"));

        // Пользовательский строй сохраняется для валидации после загрузки пресетов.
        let s = Settings {
            instrument_id: "domra".into(),
            tuning_id: Some("custom.abc-123".into()),
            ..Settings::default()
        }
        .sanitize();
        assert_eq!(s.tuning_id.as_deref(), Some("custom.abc-123"));

        // Хроматика: строя нет.
        let s = Settings {
            instrument_id: "chromatic".into(),
            tuning_id: Some("standard_e".into()),
            ..Settings::default()
        }
        .sanitize();
        assert_eq!(s.tuning_id, None);

        // Нераспознанные enum-значения не роняют файл.
        let raw = r#"{"theme": "Pink", "language": "DE", "sensitivity": "Ultra"}"#;
        let s: Settings = serde_json::from_str::<Settings>(raw).unwrap().sanitize();
        assert_eq!(s.theme, Theme::System);
        assert_eq!(s.language, Language::Ru);
        assert_eq!(s.sensitivity, Sensitivity::Normal);

        // Аудиовход: пустая строка (после trim) -> None; пробелы тримятся
        let s = Settings {
            input_device: Some("   ".into()),
            ..Settings::default()
        }
        .sanitize();
        assert_eq!(s.input_device, None);

        let s = Settings {
            input_device: Some("  USB Mic  ".into()),
            ..Settings::default()
        }
        .sanitize();
        assert_eq!(s.input_device.as_deref(), Some("USB Mic"));
    }

    fn preset(instrument: &str, id: &str) -> PresetRecord {
        PresetRecord {
            id: format!("custom.{id}"),
            instrument_id: instrument.into(),
            name: "Мой".into(),
            strings: vec![64, 69, 74],
            built_in: false,
        }
    }

    #[test]
    fn presets_round_trip_and_limit() {
        let (_dir, store) = tmp_store();
        let presets = vec![preset("domra", "a"), preset("guitar", "b")];
        store.save_presets(&presets).unwrap();
        assert_eq!(store.load_presets(), presets);

        // Лимит 20 на инструмент при сохранении.
        let mut many: Vec<PresetRecord> = (0..21)
            .map(|i| preset("guitar", &format!("n{i}")))
            .collect();
        let err = store.save_presets(&many).unwrap_err();
        assert!(
            matches!(err, StoreError::PresetLimit { ref instrument, limit: 20 } if instrument == "guitar")
        );
        // 20 — можно.
        many.pop();
        store.save_presets(&many).unwrap();
        assert_eq!(store.load_presets().len(), 20);

        // Структура файла §5.6.
        let raw = fs::read_to_string(store.presets_path()).unwrap();
        assert!(raw.contains("\"schema_version\": 3"));
        assert!(raw.contains("\"presets\":"));
    }

    #[test]
    fn invalid_presets_skipped_on_load() {
        let (_dir, store) = tmp_store();
        let file = PresetsFile {
            schema_version: SCHEMA_VERSION,
            presets: vec![
                preset("domra", "ok"),                 // валидный
                preset("unknown-instrument", "weird"), // неизвестный инструмент
                PresetRecord {
                    id: "custom.bad-midi".into(),
                    instrument_id: "guitar".into(),
                    name: "Пустой".into(),
                    strings: vec![120], // MIDI вне диапазона
                    built_in: false,
                },
                PresetRecord {
                    id: "custom.bad-len".into(),
                    instrument_id: "guitar".into(),
                    name: "Много".into(),
                    strings: vec![64, 64, 64, 64, 64, 64, 64, 64, 64], // 9 струн
                    built_in: false,
                },
            ],
        };
        fs::write(store.presets_path(), serde_json::to_string(&file).unwrap()).unwrap();
        let loaded = store.load_presets();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "custom.ok");
    }

    #[test]
    fn save_rejects_invalid_preset() {
        let (_dir, store) = tmp_store();
        // Струны вне диапазона
        let mut bad_strings = preset("guitar", "bad-strings");
        bad_strings.strings = vec![10, 200];
        assert!(matches!(
            store.save_presets(&[bad_strings]).unwrap_err(),
            StoreError::InvalidPreset { .. }
        ));
        assert!(!store.presets_path().exists());

        // Пустое имя (3.4)
        let mut bad_name = preset("guitar", "bad-name");
        bad_name.name = "   ".into();
        assert!(matches!(
            store.save_presets(&[bad_name]).unwrap_err(),
            StoreError::InvalidPreset { .. }
        ));
        assert!(!store.presets_path().exists());

        // ID без префикса custom. (3.4)
        let mut bad_id = preset("guitar", "bad-id");
        bad_id.id = "no_prefix_here".into();
        assert!(matches!(
            store.save_presets(&[bad_id]).unwrap_err(),
            StoreError::InvalidPreset { .. }
        ));
        assert!(!store.presets_path().exists());

        // Неизвестный инструмент (3.4)
        let bad_inst = preset("nonexistent_instrument", "bad-inst");
        assert!(matches!(
            store.save_presets(&[bad_inst]).unwrap_err(),
            StoreError::InvalidPreset { .. }
        ));
        assert!(!store.presets_path().exists());
    }
}
