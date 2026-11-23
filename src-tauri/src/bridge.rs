//! Bridge: состояние приложения и синхронизация движка (docs/source/03_architecture.md,
//! 3.5–3.6). Единственное место, где сходятся аудио, состояние и UI.
//!
//! `Channel<TunerState>` уходит в WebView с троттлингом ~60 Гц; сырое аудио
//! через IPC не передаётся. Ошибки устройства — значения [`AudioError`],
//! UI превращает их в состояния `error`/`nopermission`.

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use tauri::ipc::Channel;
use tuner_audio::{spawn_analyzer, AudioError, AudioSource, PlatformSource, RING_CAPACITY};
use tuner_core::engine::{Sensitivity, TunerEngine, TunerState};
use tuner_core::music::{InstrumentRegistry, Tuning};

/// Минимальный интервал отправки состояния в WebView: кап ~60 Гц (3.6).
/// Анализатор при 48 кГц даёт ~47 кадров/с — все проходят.
const SEND_MIN_INTERVAL: Duration = Duration::from_millis(15);

/// Границы опорной частоты (6.5); движок доверяет, но проверяет.
const A4_MIN: f64 = 400.0;
const A4_MAX: f64 = 480.0;

/// Сессия захвата: флаг «слушаем» и поток анализа. Под одним замком,
/// чтобы start/stop/закрытие окна не гонялись между собой.
struct Session {
    listening: bool,
    analyzer: Option<JoinHandle<()>>,
}

/// Состояние приложения: движок (общий с анализатором), источник звука
/// и сессия захвата. Настройки хранятся в конфиге движка; отдельное
/// хранилище — этап 4B (tuner-storage).
pub struct AppState {
    engine: Arc<Mutex<TunerEngine>>,
    source: Mutex<PlatformSource>,
    session: Mutex<Session>,
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
            },
            48_000,
        );
        Self {
            engine: Arc::new(Mutex::new(engine)),
            source: Mutex::new(PlatformSource::new()),
            session: Mutex::new(Session {
                listening: false,
                analyzer: None,
            }),
        }
    }

    /// Запускает захват: ring buffer → cpal → поток анализа → `on_state`.
    /// Повторный вызов переоткрывает микрофон (смена разрешения — 4.7).
    pub fn start_capture(
        &self,
        on_state: impl FnMut(TunerState) + Send + 'static,
    ) -> Result<(), AudioError> {
        let mut session = self.session.lock().expect("session");
        self.stop_locked(&mut session);

        let (producer, consumer) = rtrb::RingBuffer::new(RING_CAPACITY);
        // Порядок замков всегда session → source, взаимоблокировок нет.
        self.source.lock().expect("source").start(producer)?;
        let fs = self.source.lock().expect("source").sample_rate();

        // fs из устройства, не из константы: движок пересоберётся при отличии.
        let engine = Arc::clone(&self.engine);
        let handle = spawn_analyzer(consumer, engine, fs, on_state);
        session.listening = true;
        session.analyzer = Some(handle);
        Ok(())
    }

    /// Останавливает захват: cpal и поток анализа (3.6: окно отдельно от аудио).
    pub fn stop(&self) {
        let mut session = self.session.lock().expect("session");
        self.stop_locked(&mut session);
    }

    fn stop_locked(&self, session: &mut Session) {
        if !session.listening {
            return;
        }
        session.listening = false;
        // Дроп stream дропает producer → анализатор видит abandoned и выходит.
        self.source.lock().expect("source").stop();
        if let Some(handle) = session.analyzer.take() {
            let _ = handle.join();
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
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Состояние в WebView: ring buffer → анализатор → троттлинг → Channel.
/// Сырое аудио через IPC не идёт (3.6).
pub fn send_states_to(channel: Channel<TunerState>) -> impl FnMut(TunerState) + Send + 'static {
    let mut last_sent: Option<Instant> = None;
    move |state| {
        let now = Instant::now();
        let due = last_sent.is_none_or(|t| now.duration_since(t) >= SEND_MIN_INTERVAL);
        if due {
            last_sent = Some(now);
            let _ = channel.send(state);
        }
    }
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
        assert!(!state.session.lock().unwrap().listening);
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
        assert!(!state.session.lock().unwrap().listening);
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
}
