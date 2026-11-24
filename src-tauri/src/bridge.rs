//! Bridge: состояние приложения и синхронизация движка (docs/source/03_architecture.md,
//! 3.5–3.6). Единственное место, где сходятся аудио, состояние и UI.
//!
//! `Channel<TunerState>` уходит в WebView с троттлингом ~60 Гц; сырое аудио
//! через IPC не передаётся. Ошибки устройства — значения [`AudioError`],
//! UI превращает их в состояния `error`/`nopermission`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tauri::ipc::Channel;
use tuner_audio::{AudioError, PlatformSource, StateSink, SupervisedCapture, WatchdogConfig};
use tuner_core::engine::{Sensitivity, TunerEngine, TunerState};
use tuner_core::music::{InstrumentRegistry, Tuning};

/// Минимальный интервал отправки состояния в WebView: кап ~60 Гц (3.6).
/// Анализатор при 48 кГц даёт ~47 кадров/с — все проходят.
const SEND_MIN_INTERVAL_MS: u64 = 15;

/// Границы опорной частоты (6.5); движок доверяет, но проверяет.
const A4_MIN: f64 = 400.0;
const A4_MAX: f64 = 480.0;

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Состояние приложения: движок (общий с анализатором), живой захват
/// под наблюдением (7.2.1) и хранилище настроек/пресетов (§5.2).
/// Если каталог конфигурации недоступен, приложение работает, но настройки
/// не переживают перезапуск — это не критичный путь.
pub struct AppState {
    engine: Arc<Mutex<TunerEngine>>,
    capture: Mutex<Option<Arc<SupervisedCapture<PlatformSource>>>>,
    store: Mutex<Option<tuner_storage::Store>>,
}

/// DTO строя для команды `select_tuning`: UI присылает объект каталога
/// (встроенный или пользовательский), движку нужны только струны и привязка.
/// `name`/`built_in` в JSON игнорируются: имя — вопрос отображения, не DSP.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct TuningSpec {
    pub id: String,
    pub instrument_id: String,
    pub strings: Vec<u8>,
}

impl From<TuningSpec> for Tuning {
    fn from(spec: TuningSpec) -> Self {
        Tuning {
            id: spec.id,
            instrument_id: spec.instrument_id,
            name: tuner_core::music::LocalizedName::Custom(String::new()),
            strings: spec.strings,
            built_in: false,
        }
    }
}

impl AppState {
    /// Дефолт: гитара Standard E, A4 440, допуск 3, обычная чувствительность.
    /// Реальная частота дискретизации придёт при старте захвата.
    pub fn new() -> Self {
        let registry = InstrumentRegistry::new();
        let guitar = registry
            .tuning("guitar", "standard_e")
            .expect("standard_e есть в реестре")
            .clone();
        let engine = TunerEngine::new(
            tuner_core::engine::EngineConfig {
                tuning: Some(guitar),
                a4: 440.0,
                tolerance_cents: 3.0,
                sensitivity: Sensitivity::Normal,
                manual_string: None,
                low_range: false,
            },
            48_000,
        );
        Self {
            engine: Arc::new(Mutex::new(engine)),
            capture: Mutex::new(None),
            store: Mutex::new(None),
        }
    }

    /// Подключает хранилище (вызывается из setup, когда известен каталог).
    pub fn set_store(&self, store: tuner_storage::Store) {
        *self.store.lock().expect("store") = Some(store);
    }

    /// Запускает захват под наблюдением (7.2.1): кольцо → cpal → анализатор
    /// → `sink`; тишина дольше секунды или ошибка потока — переоткрытие.
    /// Повторный вызов переоткрывает микрофон (смена разрешения — 4.7).
    pub fn start_capture(&self, sink: StateSink) -> Result<(), AudioError> {
        let mut slot = self.capture.lock().expect("capture");
        // Предыдущая сессия, если была, останавливается полностью.
        if let Some(previous) = slot.take() {
            previous.stop();
        }
        let capture = SupervisedCapture::spawn(
            Arc::clone(&self.engine),
            // Каждое (пере)открытие — новый источник: поток-владелец
            // устройства пересоздаётся, устройство освобождается.
            Arc::new(PlatformSource::new),
            sink,
            WatchdogConfig::default(),
        )?;
        *slot = Some(capture);
        Ok(())
    }

    /// Останавливает захват: источник, анализатор, наблюдатель
    /// (3.6: окно отдельно от аудио; закрытие окна зовёт то же).
    pub fn stop(&self) {
        if let Some(capture) = self.capture.lock().expect("capture").take() {
            capture.stop();
        }
    }

    /// Смена конфига движка под замком: одна точка входа для команд.
    fn with_config(&self, update: impl FnOnce(&mut tuner_core::engine::EngineConfig)) {
        let mut engine = self.engine.lock().expect("engine");
        let mut config = engine.config().clone();
        update(&mut config);
        engine.set_config(config);
    }

    /// Опорная частота A4: границы 400–480 (6.5).
    pub fn set_reference(&self, hz: f64) {
        let hz = hz.clamp(A4_MIN, A4_MAX);
        self.with_config(|c| c.a4 = hz);
    }

    /// Инструмент: первый строй из реестра; хроматика — строй `None` (D-6).
    /// Неизвестный ID игнорируется. Выбранная струна сбрасывается.
    pub fn select_instrument(&self, id: &str) {
        let registry = InstrumentRegistry::new();
        if id != "chromatic" && !registry.instruments().iter().any(|i| i.id == id) {
            return;
        }
        let tuning = if id == "chromatic" {
            None
        } else {
            registry.tunings_for(id).first().map(|t| (*t).clone())
        };
        self.with_config(|c| {
            c.tuning = tuning;
            c.manual_string = None;
        });
    }

    /// Строй целиком (встроенный или пользовательский): UI передаёт объект,
    /// отдельного реестра пользовательских строёв в bridge нет (4B — storage).
    pub fn select_tuning(&self, spec: TuningSpec) {
        self.with_config(|c| {
            c.tuning = Some(Tuning::from(spec));
            c.manual_string = None;
        });
    }

    /// Ручная струна (ФТ-15): `None` — автоопределение с гистерезисом.
    pub fn select_string(&self, index: Option<usize>) {
        self.with_config(|c| c.manual_string = index);
    }

    /// Допуск «в строе» в центах (2/3/5; defensive clamp).
    pub fn set_tolerance(&self, cents: f32) {
        let cents = cents.clamp(0.5, 10.0);
        self.with_config(|c| c.tolerance_cents = cents);
    }

    /// Чувствительность входного гейта.
    pub fn set_sensitivity(&self, sensitivity: Sensitivity) {
        self.with_config(|c| c.sensitivity = sensitivity);
    }

    /// «Низкий диапазон»: окно хроматического режима (4.2.3).
    pub fn set_low_range(&self, low_range: bool) {
        self.with_config(|c| c.low_range = low_range);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Настройки для UI (camelCase-поля, lowercase-значения — так устроен
/// `ui/src/lib/types.ts`): зеркало `tuner_storage::Settings` (§5.3).
/// `schema_version` живёт только в файле, через UI не ходит.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub reference_hz: f64,
    pub instrument_id: String,
    pub tuning_id: Option<String>,
    pub notation: String,
    pub theme: String,
    pub language: String,
    pub haptics: bool,
    pub sound: bool,
    pub tolerance_cents: u8,
    pub sensitivity: String,
    pub low_range: bool,
    pub big_type: bool,
    pub onboarding_done: bool,
    #[serde(default)]
    pub input_device: Option<String>,
}

impl From<tuner_storage::Settings> for SettingsDto {
    fn from(s: tuner_storage::Settings) -> Self {
        Self {
            reference_hz: s.reference_hz,
            instrument_id: s.instrument_id,
            tuning_id: s.tuning_id,
            notation: match s.notation {
                tuner_storage::Notation::Solfeggio => "solfeggio".into(),
                _ => "latin".into(),
            },
            theme: match s.theme {
                tuner_storage::Theme::Light => "light".into(),
                tuner_storage::Theme::Dark => "dark".into(),
                _ => "system".into(),
            },
            language: match s.language {
                tuner_storage::Language::En => "en".into(),
                _ => "ru".into(),
            },
            haptics: s.haptics,
            sound: s.sound,
            tolerance_cents: s.tolerance_cents,
            sensitivity: match s.sensitivity {
                tuner_storage::Sensitivity::Low => "low".into(),
                tuner_storage::Sensitivity::High => "high".into(),
                _ => "normal".into(),
            },
            low_range: s.low_range,
            big_type: s.big_type,
            onboarding_done: s.onboarding_done,
            input_device: s.input_device,
        }
    }
}

impl From<SettingsDto> for tuner_storage::Settings {
    fn from(dto: SettingsDto) -> Self {
        // Незнакомые значения UI → дефолт; `sanitize` всё равно подстрахует.
        Self {
            schema_version: tuner_storage::SCHEMA_VERSION,
            reference_hz: dto.reference_hz,
            instrument_id: dto.instrument_id,
            tuning_id: dto.tuning_id,
            notation: match dto.notation.as_str() {
                "solfeggio" => tuner_storage::Notation::Solfeggio,
                _ => tuner_storage::Notation::Latin,
            },
            theme: match dto.theme.as_str() {
                "light" => tuner_storage::Theme::Light,
                "dark" => tuner_storage::Theme::Dark,
                _ => tuner_storage::Theme::System,
            },
            language: match dto.language.as_str() {
                "en" => tuner_storage::Language::En,
                _ => tuner_storage::Language::Ru,
            },
            haptics: dto.haptics,
            sound: dto.sound,
            tolerance_cents: dto.tolerance_cents,
            sensitivity: match dto.sensitivity.as_str() {
                "low" => tuner_storage::Sensitivity::Low,
                "high" => tuner_storage::Sensitivity::High,
                _ => tuner_storage::Sensitivity::Normal,
            },
            low_range: dto.low_range,
            big_type: dto.big_type,
            onboarding_done: dto.onboarding_done,
            input_device: dto.input_device,
        }
    }
}

/// Пользовательский строй для UI (§5.6): имя — плоская строка,
/// `built_in` у пресетов всегда false, UI оборачивает имя в `Custom`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDto {
    pub id: String,
    pub instrument_id: String,
    pub name: String,
    pub strings: Vec<u8>,
}

impl From<tuner_storage::PresetRecord> for PresetDto {
    fn from(p: tuner_storage::PresetRecord) -> Self {
        Self {
            id: p.id,
            instrument_id: p.instrument_id,
            name: p.name,
            strings: p.strings,
        }
    }
}

impl From<PresetDto> for tuner_storage::PresetRecord {
    fn from(dto: PresetDto) -> Self {
        Self {
            id: dto.id,
            instrument_id: dto.instrument_id,
            name: dto.name,
            strings: dto.strings,
            built_in: false,
        }
    }
}

impl AppState {
    /// Настройки для UI: из файла или дефолт — чтение не падает никогда.
    pub fn settings_dto(&self) -> SettingsDto {
        let store = self.store.lock().expect("store");
        match store.as_ref() {
            Some(store) => store.load_settings().into(),
            None => tuner_storage::Settings::default().into(),
        }
    }

    /// Сохранение настроек: ошибки логируются, не роняют приложение.
    pub fn save_settings_dto(&self, dto: SettingsDto) {
        let store = self.store.lock().expect("store");
        let Some(store) = store.as_ref() else {
            log::warn!("хранилище недоступно: настройки не сохранены");
            return;
        };
        if let Err(e) = store.save_settings(&dto.into()) {
            log::warn!("настройки не сохранены: {e}");
        }
    }

    /// Пользовательские строи для UI; нет хранилища — пусто.
    pub fn presets_dto(&self) -> Vec<PresetDto> {
        let store = self.store.lock().expect("store");
        match store.as_ref() {
            Some(store) => store.load_presets().into_iter().map(Into::into).collect(),
            None => Vec::new(),
        }
    }

    /// Создать/обновить пресет; ошибки (лимит 20, битые струны) — в лог.
    pub fn save_preset_dto(&self, dto: PresetDto) {
        let store = self.store.lock().expect("store");
        let Some(store) = store.as_ref() else {
            log::warn!("хранилище недоступно: пресет не сохранён");
            return;
        };
        let record = tuner_storage::PresetRecord::from(dto);
        let mut presets = store.load_presets();
        match presets.iter().position(|p| p.id == record.id) {
            Some(i) => presets[i] = record,
            None => presets.push(record),
        }
        if let Err(e) = store.save_presets(&presets) {
            log::warn!("пресет не сохранён: {e}");
        }
    }

    /// Удалить пресет по ID; встроенные строи здесь не проходят никогда
    /// (UI не даёт), но и файл не содержит.
    pub fn delete_preset(&self, id: &str) {
        let store = self.store.lock().expect("store");
        let Some(store) = store.as_ref() else {
            log::warn!("хранилище недоступно: пресет не удалён");
            return;
        };
        let mut presets = store.load_presets();
        let before = presets.len();
        presets.retain(|p| p.id != id);
        if presets.len() == before {
            return; // не было такого — файл не трогаем
        }
        if let Err(e) = store.save_presets(&presets) {
            log::warn!("пресеты не сохранены: {e}");
        }
    }
}

/// Состояния в WebView через `Channel`: троттлинг ≥ 15 мс (кап ~60 Гц, 3.6),
/// сырое аудио через IPC не идёт. Замер — атомарный: sink общий для
/// переоткрытий потока, поэтомуFn, не FnMut.
pub fn channel_sink(channel: Channel<TunerState>) -> StateSink {
    let last_sent_ms = AtomicU64::new(0);
    Arc::new(move |state: TunerState| {
        let now = now_ms();
        let last = last_sent_ms.load(Ordering::Relaxed);
        if last == 0 || now.saturating_sub(last) >= SEND_MIN_INTERVAL_MS {
            last_sent_ms.store(now, Ordering::Relaxed);
            let _ = channel.send(state);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tuner_core::engine::EngineConfig;

    fn config(state: &AppState) -> EngineConfig {
        state.engine.lock().unwrap().config().clone()
    }

    #[test]
    fn default_is_guitar_standard_440() {
        let state = AppState::new();
        let c = config(&state);
        assert_eq!(c.tuning.as_ref().unwrap().id, "standard_e");
        assert!((c.a4 - 440.0).abs() < 1e-9);
        assert_eq!(c.tolerance_cents, 3.0);
        assert_eq!(c.sensitivity, Sensitivity::Normal);
        assert_eq!(c.manual_string, None);
        assert!(state.capture.lock().unwrap().is_none());
    }

    #[test]
    fn reference_clamped_to_400_480() {
        let state = AppState::new();
        state.set_reference(442.0);
        assert!((config(&state).a4 - 442.0).abs() < 1e-9);
        state.set_reference(999.0);
        assert!((config(&state).a4 - 480.0).abs() < 1e-9);
        state.set_reference(300.0);
        assert!((config(&state).a4 - 400.0).abs() < 1e-9);
    }

    #[test]
    fn instrument_switch_resets_string_and_tuning() {
        let state = AppState::new();
        state.select_string(Some(2));
        state.select_instrument("violin");
        let c = config(&state);
        assert_eq!(c.manual_string, None);
        assert_eq!(c.tuning.as_ref().unwrap().id, "violin_std");

        // Хроматика: строя нет, струн нет (D-6).
        state.select_instrument("chromatic");
        assert!(config(&state).tuning.is_none());

        // Неизвестный инструмент не ломает состояние.
        state.select_instrument("no-such-instrument");
        assert!(config(&state).tuning.is_none());
    }

    #[test]
    fn custom_tuning_object_overrides() {
        let state = AppState::new();
        let spec = TuningSpec {
            id: "custom.1".into(),
            instrument_id: "domra".into(),
            strings: vec![55, 62, 69],
        };
        state.select_tuning(spec);
        let c = config(&state);
        assert_eq!(c.tuning.as_ref().unwrap().strings, vec![55, 62, 69]);
        // Окно по нижней ноте: G3 ≥ E2 → 4096.
        assert_eq!(state.engine.lock().unwrap().window(), 4096);
    }

    #[test]
    fn tolerance_and_sensitivity_update_engine() {
        let state = AppState::new();
        state.set_tolerance(5.0);
        assert_eq!(config(&state).tolerance_cents, 5.0);
        state.set_tolerance(100.0); // clamp
        assert_eq!(config(&state).tolerance_cents, 10.0);
        state.set_sensitivity(Sensitivity::High);
        assert_eq!(config(&state).sensitivity, Sensitivity::High);
    }

    #[test]
    fn stop_on_fresh_state_is_noop() {
        let state = AppState::new();
        state.stop();
        assert!(state.capture.lock().unwrap().is_none());
        // Повторный stop — тоже без паники.
        state.stop();
    }

    #[test]
    fn select_string_toggles_manual() {
        let state = AppState::new();
        state.select_string(Some(1));
        assert_eq!(config(&state).manual_string, Some(1));
        state.select_string(None);
        assert_eq!(config(&state).manual_string, None);
    }

    /// Полный цикл настроек через DTO и реальное хранилище во временном каталоге.
    #[test]
    fn settings_round_trip_via_dto_and_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = AppState::new();
        state.set_store(tuner_storage::Store::open(dir.path()).expect("store"));
        assert_eq!(
            state.settings_dto(),
            SettingsDto::from(tuner_storage::Settings::default())
        );

        let mut dto = state.settings_dto();
        dto.reference_hz = 442.0;
        dto.instrument_id = "bass".into();
        dto.tuning_id = Some("bass5".into());
        dto.language = "en".into();
        dto.theme = "light".into();
        dto.notation = "solfeggio".into();
        dto.tolerance_cents = 5;
        dto.sensitivity = "high".into();
        dto.low_range = true;
        dto.big_type = true;
        dto.onboarding_done = true;
        state.save_settings_dto(dto.clone());
        assert_eq!(state.settings_dto(), dto, "файл переживает запись и чтение");
    }

    #[test]
    fn presets_crud_via_dto_and_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = AppState::new();
        state.set_store(tuner_storage::Store::open(dir.path()).expect("store"));
        let preset = PresetDto {
            id: "custom.1".into(),
            instrument_id: "domra".into(),
            name: "Мой".into(),
            strings: vec![64, 69, 74],
        };
        state.save_preset_dto(preset.clone());
        assert_eq!(state.presets_dto(), vec![preset.clone()]);
        // Обновление по ID не плодит дубликаты.
        let mut edited = preset.clone();
        edited.strings = vec![55, 62, 69];
        state.save_preset_dto(edited.clone());
        assert_eq!(state.presets_dto(), vec![edited.clone()]);
        // Удаление.
        state.delete_preset(&edited.id);
        assert!(state.presets_dto().is_empty());
        // Удаление несуществующего — не паника и не запись файла.
        state.delete_preset("custom.nothing");
        assert!(state.presets_dto().is_empty());
    }

    /// Нет хранилища (каталог недоступен) — приложение работает: дефолты,
    /// сохранение молча логируется, ничего не падает.
    #[test]
    fn storage_degrades_gracefully_without_store() {
        let state = AppState::new();
        assert_eq!(
            state.settings_dto(),
            SettingsDto::from(tuner_storage::Settings::default())
        );
        state.save_settings_dto(state.settings_dto());
        assert!(state.presets_dto().is_empty());
        state.save_preset_dto(PresetDto {
            id: "custom.x".into(),
            instrument_id: "guitar".into(),
            name: "Мой".into(),
            strings: vec![64],
        });
        state.delete_preset("custom.x");
        assert!(state.presets_dto().is_empty());
    }
}
