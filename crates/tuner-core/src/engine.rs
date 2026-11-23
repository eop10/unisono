//! Движок тюнера (docs/source/03_architecture.md, 3.5; 04_dsp.md, 4.4).
//!
//! Оркестрация кадра: детектор → сглаживание → выбор струны → «в строе».
//! Не знает про аудио-колбэки, Tauri и UI: на входе кадр `f32`, на выходе
//! [`TunerState`] для IPC. Состояния `Error`/`NoPermission` — это ошибки
//! аудио-слоя, не движка: их подставляют bridge и watchdog (`tuner-audio`) из `AudioError`.

use crate::dsp::{level_db, DetectorConfig, Estimate, PitchDetector, SmoothingFilter, GATE_DB};
use crate::music::{note, Tuning};

/// Шаг между кадрами (отсчёты, 4.2.1): ~47 обновлений/с при 48 кГц.
/// Анализатор читает ring buffer с этим шагом независимо от размера окна.
pub const HOP: usize = 1024;

/// Удержание «в строе» (мс, ФТ-07, 4.2.1).
const INTUNE_HOLD_MS: f32 = 250.0;
/// Гистерезис автоопределения: выигрыш новой струны для переключения (4.3.6).
const SWITCH_WIN_CENTS: f32 = 10.0;
/// Выход из текущей струны по отклонению (4.3.6).
const SWITCH_LEAVE_CENTS: f32 = 60.0;
/// «Чужая нота» в ручном режиме: дальше половины полутона (4.7).
const FOREIGN_CENTS: f32 = 50.0;

/// Состояние тюнера для UI (4.4). Сериализуется в camelCase — как ожидает Svelte.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunerState {
    /// `silent` | `uncertain` | `tracking` | `intune`.
    pub kind: TunerKind,
    /// Измеренная частота (Гц); `None` — оценки нет.
    pub f0: Option<f32>,
    /// Отклонение от цели в центах; `None` — не показываем.
    pub cents: Option<f32>,
    /// Достоверность последнего кадра NSDF.
    pub clarity: f32,
    /// Уровень кадра, dBFS — для индикатора входа (ФТ-16).
    pub db: f32,
    /// Нота цели: струна в режимах «по струнам», ближайшая — в хроматике.
    pub note_midi: Option<u8>,
    /// Целевая частота (Гц) при текущей A4.
    pub target_hz: Option<f32>,
    /// Индексы активных струн; унисон-пара целиком (D-7); в хроматике пусто.
    pub string_indices: Vec<usize>,
}

/// Kind-состояния из 4.4. `Error`/`NoPermission` — состояния аудио-слоя:
/// сам движок их не создаёт, но канал в UI несёт (7.2.1: watchdog
/// сообщает, что захват умер и не восстановился).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TunerKind {
    Silent,
    Uncertain,
    Tracking,
    InTune,
    /// Микрофон занят или недоступен (4.4).
    Error,
    /// Нет разрешения на микрофон (4.4, TCC).
    NoPermission,
}

/// Чувствительность ввода (6.6, §5.3): порог достоверности `clarity`,
/// при котором показывается цифра. Порог тишины при этом фиксирован (§4.2.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sensitivity {
    Low,
    #[default]
    Normal,
    High,
}

impl Sensitivity {
    /// Минимальная достоверность для показа цифры (§5.3): 0,75 / 0,60 / 0,45.
    pub fn clarity_min(self) -> f32 {
        match self {
            Sensitivity::Low => 0.75,
            Sensitivity::Normal => 0.60,
            Sensitivity::High => 0.45,
        }
    }
}

/// Конфигурация движка. `tuning = None` — хроматический режим (D-6);
/// `manual_string = Some(i)` — струна, выбранная тапом (ФТ-15).
#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub tuning: Option<Tuning>,
    /// Опорная частота A4, Гц (400–480).
    pub a4: f64,
    /// Допуск «в строе» в центах (2/3/5).
    pub tolerance_cents: f32,
    pub sensitivity: Sensitivity,
    /// Индекс выбранной струны; `None` — автоопределение.
    pub manual_string: Option<usize>,
    /// «Низкий диапазон» (5.3): окно 16384 в хроматическом режиме (4.2.3).
    pub low_range: bool,
}

/// Результат сопоставления частоты с целью.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchOutcome {
    /// Индексы струн цели; унисон-пара подсвечивается целиком (6.3.6).
    pub string_indices: Vec<usize>,
    /// Нота цели (MIDI).
    pub note_midi: u8,
    /// Целевая частота при текущей A4 (Гц).
    pub target_hz: f32,
    /// Отклонение `1200·log2(f0/target)`, центы.
    pub cents: f32,
    /// Ручной режим, чужая нота (|cents| > 50): струны не подсвечиваются.
    pub foreign: bool,
}

/// Выбор целевой струны (3.5): авто с гистерезисом, ручной, хроматический.
/// Состояние — только текущая струна автоопределения.
#[derive(Clone, Debug, Default)]
pub struct StringMatcher {
    current: Option<usize>,
}

impl StringMatcher {
    pub fn reset(&mut self) {
        self.current = None;
    }

    /// Цель для частоты `f0` (Гц): хроматика при `tuning = None`,
    /// ручная струна при `manual_string`, иначе авто с гистерезисом.
    pub fn match_f0(&mut self, f0: f64, config: &EngineConfig) -> MatchOutcome {
        match (&config.tuning, config.manual_string) {
            (None, _) => self.chromatic(f0, config.a4),
            (Some(tuning), Some(index)) => self.manual(f0, tuning, config.a4, index),
            (Some(tuning), None) => self.auto(f0, tuning, config.a4),
        }
    }

    /// Хроматический режим: ближайшая нота равномерной темперации, без струн.
    fn chromatic(&self, f0: f64, a4: f64) -> MatchOutcome {
        let midi = note::nearest_midi(f0, a4);
        let target = note::hz(midi, a4);
        MatchOutcome {
            string_indices: Vec::new(),
            note_midi: midi,
            target_hz: target as f32,
            cents: note::cents(f0, target) as f32,
            foreign: false,
        }
    }

    /// Ручной режим: выбранная струна; чужая нота гасит подсветку пары (4.7).
    fn manual(&mut self, f0: f64, tuning: &Tuning, a4: f64, index: usize) -> MatchOutcome {
        match self.string_outcome(f0, tuning, a4, index) {
            Some(mut outcome) => {
                if outcome.cents.abs() > FOREIGN_CENTS {
                    outcome.foreign = true;
                    outcome.string_indices.clear();
                }
                self.current = Some(index);
                outcome
            }
            // Индекс вне строя (строй сменился) — ведём себя как авто.
            None => self.auto(f0, tuning, a4),
        }
    }

    /// Автоопределение: ближайшая струна по |cents| с гистерезисом (4.3.6):
    /// не переключаемся при выигрыше новой < 10 центов; текущую покидаем,
    /// когда она ушла за ±60 центов. Унисон-пара отдаётся целиком (D-7).
    fn auto(&mut self, f0: f64, tuning: &Tuning, a4: f64) -> MatchOutcome {
        if tuning.strings.is_empty() {
            return self.chromatic(f0, a4);
        }
        let dev: Vec<f64> = tuning
            .strings
            .iter()
            .map(|&m| note::cents(f0, note::hz(m, a4)).abs())
            .collect();
        let best = (0..dev.len())
            .min_by(|&i, &j| dev[i].total_cmp(&dev[j]))
            .expect("strings не пуста");
        let mut outcome = self
            .string_outcome(f0, tuning, a4, best)
            .expect("best < len");
        if let Some(current) = self.current {
            if let Some(cur) = self.string_outcome(f0, tuning, a4, current) {
                let wins = cur.cents.abs() - outcome.cents.abs();
                let leave = cur.cents.abs() > SWITCH_LEAVE_CENTS;
                if !(wins >= SWITCH_WIN_CENTS || leave) {
                    outcome = cur;
                } else {
                    self.current = Some(best);
                }
            } else {
                self.current = Some(best);
            }
        } else {
            self.current = Some(best);
        }
        outcome
    }

    /// Цель-струна `index` с унисон-парой; `None` для индекса вне строя.
    fn string_outcome(
        &self,
        f0: f64,
        tuning: &Tuning,
        a4: f64,
        index: usize,
    ) -> Option<MatchOutcome> {
        let &midi = tuning.strings.get(index)?;
        let target = note::hz(midi, a4);
        Some(MatchOutcome {
            string_indices: tuning.string_indices(midi),
            note_midi: midi,
            target_hz: target as f32,
            cents: note::cents(f0, target) as f32,
            foreign: false,
        })
    }
}

/// Последнее устойчивое показание: в `Uncertain` нота остаётся приглушённой,
/// игла — в последней позиции (6.3.4).
#[derive(Clone, Debug)]
struct LastReading {
    f0: f32,
    cents: f32,
    note_midi: u8,
    target_hz: f32,
    string_indices: Vec<usize>,
}

/// Движок тюнера: детектор + сглаживание + матчер + удержание «в строе».
pub struct TunerEngine {
    config: EngineConfig,
    fs: u32,
    detector: PitchDetector,
    smoother: SmoothingFilter,
    matcher: StringMatcher,
    /// Сколько мс подряд |cents| ≤ допуска.
    intune_ms: f32,
    last: Option<LastReading>,
}

impl TunerEngine {
    pub fn new(config: EngineConfig, fs: u32) -> Self {
        Self {
            detector: PitchDetector::new(detector_config(&config, fs)),
            config,
            fs,
            smoother: SmoothingFilter::new(),
            matcher: StringMatcher::default(),
            intune_ms: 0.0,
            last: None,
        }
    }

    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Окно анализа текущего строя (отсчёты) — им пользуется поток анализа.
    pub fn window(&self) -> usize {
        self.detector.config().window
    }

    /// Смена конфигурации: при смене строя/A4/чувствительности пересобирает
    /// `DetectorConfig` (окно из реестра, f_min из фактической цели);
    /// сглаживание и матчер сбрасываются только при смене строя — смена A4
    /// не требует пересчёта истории (4.3.5).
    pub fn set_config(&mut self, config: EngineConfig) {
        let tuning_changed = self.config.tuning != config.tuning;
        let manual_changed = self.config.manual_string != config.manual_string;
        let detector_changed = detector_config(&config, self.fs) != *self.detector.config();
        self.config = config;
        if tuning_changed {
            self.smoother.reset();
            self.matcher.reset();
            self.last = None;
        }
        if tuning_changed || manual_changed {
            // Новая цель: удержание «в строе» начинается заново.
            self.intune_ms = 0.0;
        }
        if detector_changed {
            self.detector = PitchDetector::new(detector_config(&self.config, self.fs));
        }
    }

    /// Полный сброс адаптивного состояния (смена устройства, перезапуск).
    pub fn reset(&mut self) {
        self.smoother.reset();
        self.matcher.reset();
        self.intune_ms = 0.0;
        self.last = None;
    }

    /// Обрабатывает кадр длиной ≥ [`Self::window`]; возвращает состояние для UI.
    ///
    /// `Estimate::Silent`/`None` → `silent`; `Uncertain` → `uncertain`
    /// (последнее показание остаётся приглушённым); `Pitch` → `tracking`/`intune`.
    pub fn process_frame(&mut self, raw: &mut [f32], fs: u32) -> TunerState {
        if fs != self.fs {
            // Смена частоты дискретизации устройства: параметры от фактической fs (4.7).
            self.fs = fs;
            self.detector = PitchDetector::new(detector_config(&self.config, fs));
            self.reset();
        }
        let dt_ms = HOP as f64 * 1000.0 / f64::from(fs);
        let window = self.detector.config().window;
        let estimate = self.detector.process_frame(raw, fs);
        // Уровень нужен и без оценки — индикатор входа живёт в «Слушаю…» (ФТ-16).
        let db = match estimate {
            Some(Estimate::Pitch { db, .. }) => db,
            _ if raw.len() >= window => level_db(&raw[..window]),
            _ => -180.0,
        };

        match estimate {
            None | Some(Estimate::Silent) => {
                let _ = self.smoother.push(None, dt_ms);
                self.intune_ms = 0.0;
                self.last = None;
                TunerState {
                    kind: TunerKind::Silent,
                    f0: None,
                    cents: None,
                    clarity: 0.0,
                    db,
                    note_midi: None,
                    target_hz: None,
                    string_indices: Vec::new(),
                }
            }
            Some(Estimate::Uncertain { clarity }) => {
                let _ = self.smoother.push(None, dt_ms);
                self.intune_ms = 0.0;
                // Последняя нота приглушена, игла в последней позиции (6.3.4).
                let last = self.last.as_ref();
                TunerState {
                    kind: TunerKind::Uncertain,
                    f0: last.map(|l| l.f0),
                    cents: last.map(|l| l.cents),
                    clarity,
                    db,
                    note_midi: last.map(|l| l.note_midi),
                    target_hz: last.map(|l| l.target_hz),
                    string_indices: last.map(|l| l.string_indices.clone()).unwrap_or_default(),
                }
            }
            Some(Estimate::Pitch { f0, clarity, .. }) => {
                let Some(f) = self.smoother.push(Some(f64::from(f0)), dt_ms) else {
                    // Недостижимо для f0 > 0; защищаемся без паники в аудио-пути.
                    return TunerState {
                        kind: TunerKind::Uncertain,
                        f0: None,
                        cents: None,
                        clarity,
                        db,
                        note_midi: None,
                        target_hz: None,
                        string_indices: Vec::new(),
                    };
                };
                let outcome = self.matcher.match_f0(f, &self.config);

                if outcome.foreign {
                    // Ручной режим, чужая нота: пара не подсвечивается (4.7).
                    self.intune_ms = 0.0;
                    let last = self.last.as_ref();
                    return TunerState {
                        kind: TunerKind::Uncertain,
                        f0: last.map(|l| l.f0),
                        cents: last.map(|l| l.cents),
                        clarity,
                        db,
                        note_midi: last.map(|l| l.note_midi),
                        target_hz: last.map(|l| l.target_hz),
                        string_indices: Vec::new(),
                    };
                }

                // Смена цели (струны/ноты) перезапускает удержание «в строе».
                let target_changed = self.last.as_ref().is_none_or(|l| {
                    l.note_midi != outcome.note_midi || l.string_indices != outcome.string_indices
                });
                if target_changed {
                    self.intune_ms = 0.0;
                }
                if outcome.cents.abs() <= self.config.tolerance_cents {
                    self.intune_ms += dt_ms as f32;
                } else {
                    self.intune_ms = 0.0;
                }
                let kind = if self.intune_ms >= INTUNE_HOLD_MS {
                    TunerKind::InTune
                } else {
                    TunerKind::Tracking
                };
                self.last = Some(LastReading {
                    f0: f as f32,
                    cents: outcome.cents,
                    note_midi: outcome.note_midi,
                    target_hz: outcome.target_hz,
                    string_indices: outcome.string_indices.clone(),
                });
                TunerState {
                    kind,
                    f0: Some(f as f32),
                    cents: Some(outcome.cents),
                    clarity,
                    db,
                    note_midi: Some(outcome.note_midi),
                    target_hz: Some(outcome.target_hz),
                    string_indices: outcome.string_indices,
                }
            }
        }
    }
}

/// Конфигурация детектора из конфига движка: окно по строю (реестр),
/// `f_min` — фактическая нижняя цель при текущей A4. Порог тишины —
/// фиксированный §4.2.1; чувствительность двигает `clarity_min` (§5.3).
fn detector_config(config: &EngineConfig, fs: u32) -> DetectorConfig {
    let mut cfg = match &config.tuning {
        Some(tuning) => DetectorConfig::for_tuning(tuning, config.a4, fs),
        // Хроматика: 4096, с «Низким диапазоном» — 16384 (4.2.3, D-3).
        None => DetectorConfig::for_chromatic(config.low_range, fs),
    };
    cfg.gate_db = GATE_DB;
    cfg.clarity_min = config.sensitivity.clarity_min();
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::registry::InstrumentRegistry;
    use crate::music::tunings::LocalizedName;

    const FS: u32 = 48_000;

    fn config(tuning: Option<Tuning>, a4: f64) -> EngineConfig {
        EngineConfig {
            tuning,
            a4,
            tolerance_cents: 3.0,
            sensitivity: Sensitivity::Normal,
            manual_string: None,
            low_range: false,
        }
    }

    fn engine(tuning: Option<Tuning>, a4: f64) -> TunerEngine {
        TunerEngine::new(config(tuning, a4), FS)
    }

    fn guitar() -> Tuning {
        InstrumentRegistry::new()
            .tuning("guitar", "standard_e")
            .unwrap()
            .clone()
    }

    fn balalaika() -> Tuning {
        InstrumentRegistry::new()
            .tuning("balalaika", "balalaika_prima")
            .unwrap()
            .clone()
    }

    fn sine(freq: f64, amp: f32) -> Vec<f32> {
        (0..4096)
            .map(|i| {
                let t = f64::from(i) / f64::from(FS);
                (f64::from(amp) * (2.0 * std::f64::consts::PI * freq * t).sin()) as f32
            })
            .collect()
    }

    /// Прогон `n` кадров и итоговое состояние (окно гитары 4096).
    fn run_frames(engine: &mut TunerEngine, freq: f64, n: usize) -> TunerState {
        let mut state = None;
        for _ in 0..n {
            let mut frame = sine(freq, 0.1);
            state = Some(engine.process_frame(&mut frame, FS));
        }
        state.expect("хотя бы один кадр")
    }

    #[test]
    fn e2_seven_cents_flat() {
        let target = note::hz(40, 440.0); // E2 = 82,41 Гц
        let f = target * 2f64.powf(-7.0 / 1200.0);
        let mut e = engine(Some(guitar()), 440.0);
        let st = run_frames(&mut e, f, 1);
        assert_eq!(st.kind, TunerKind::Tracking);
        let c = st.cents.expect("центы видны");
        assert!((c + 7.0).abs() < 1.0, "cents = {c}");
        assert_eq!(st.note_midi, Some(40));
        assert!((st.target_hz.expect("target") - target as f32).abs() < 0.01);
        assert_eq!(st.string_indices, vec![0]);
    }

    #[test]
    fn intune_after_250ms_and_reset_on_exit() {
        let f = note::hz(40, 440.0);
        let mut e = engine(Some(guitar()), 440.0);
        // 11 кадров = 234,7 мс < 250 мс — ещё не «в строе».
        for i in 0..11 {
            let st = run_frames(&mut e, f, 1);
            assert_eq!(st.kind, TunerKind::Tracking, "кадр {i}");
        }
        // 12-й кадр: 256 мс ≥ 250 мс.
        let st = run_frames(&mut e, f, 1);
        assert_eq!(st.kind, TunerKind::InTune);

        // Выход за допуск: +15 центов > 3 — сразу в Tracking (4.4), без задержки.
        let f15 = f * 2f64.powf(15.0 / 1200.0);
        for i in 0..25 {
            let st = run_frames(&mut e, f15, 1);
            if i > 1 {
                assert_eq!(st.kind, TunerKind::Tracking, "кадр {i} после выхода");
            }
        }
        // Возврат в допуск: EMA возвращается к нулю, затем 250 мс удержания —
        // снова «в строе».
        let mut intune_at = None;
        for i in 0..25 {
            if run_frames(&mut e, f, 1).kind == TunerKind::InTune {
                intune_at = Some(i);
                break;
            }
        }
        assert!(
            intune_at.is_some(),
            "после возврата в допуск — снова «в строе»"
        );
        // Синус 0,1: −23 dBFS минус переход ФВЧ в начале кадра → ≈ −24,5.
        assert!(st.db > -30.0 && st.db < -20.0, "db = {}", st.db);
    }

    #[test]
    fn silence_is_listening() {
        let mut e = engine(Some(guitar()), 440.0);
        run_frames(&mut e, note::hz(40, 440.0), 3);
        let mut frame = vec![0f32; 4096];
        let st = e.process_frame(&mut frame, FS);
        assert_eq!(st.kind, TunerKind::Silent);
        assert_eq!(st.f0, None);
        assert_eq!(st.cents, None);
        assert_eq!(st.note_midi, None);
        assert_eq!(st.string_indices, Vec::<usize>::new());
    }

    #[test]
    fn short_frame_is_uncertain() {
        let mut e = engine(Some(guitar()), 440.0);
        let mut frame = vec![0f32; 100];
        let st = e.process_frame(&mut frame, FS);
        assert_eq!(st.kind, TunerKind::Uncertain);
        assert_eq!(st.f0, None);
    }

    /// Тесный строй E2–F2 (100 центов): переключение только при выигрыше ≥ 10.
    fn two_string_tuning() -> Tuning {
        Tuning {
            id: "two".into(),
            instrument_id: "guitar".into(),
            name: LocalizedName::Custom("two".into()),
            strings: vec![40, 41],
            built_in: false,
        }
    }

    #[test]
    fn string_switch_hysteresis() {
        let e2 = note::hz(40, 440.0);
        let mut e = engine(Some(two_string_tuning()), 440.0);

        // Устойчиво на струне 0.
        let st = run_frames(&mut e, e2, 25);
        assert_eq!(st.string_indices, vec![0]);

        // E2+54: струна 1 ближе на 8 центов (< 10) — остаёмся на 0 (4.3.6).
        let st = run_frames(&mut e, e2 * 2f64.powf(54.0 / 1200.0), 25);
        assert_eq!(
            st.string_indices,
            vec![0],
            "выигрыш < 10 — не переключаемся"
        );
        assert!(
            (st.cents.unwrap() - 54.0).abs() < 2.0,
            "cents = {}",
            st.cents.unwrap()
        );

        // E2+56: струна 1 выигрывает 12 центов — переключение.
        let st = run_frames(&mut e, e2 * 2f64.powf(56.0 / 1200.0), 25);
        assert_eq!(st.string_indices, vec![1], "выигрыш ≥ 10 — переключаемся");
        assert!((st.cents.unwrap() + 44.0).abs() < 2.0);

        // E2+95: текущая далеко за ±60 — берём ближайшую (строка 1, −5 центов).
        let st = run_frames(&mut e, e2 * 2f64.powf(95.0 / 1200.0), 25);
        assert_eq!(st.string_indices, vec![1]);
        assert!(st.cents.unwrap().abs() < 10.0);
    }

    #[test]
    fn balalaika_unison_pair_in_auto() {
        let e4 = note::hz(64, 440.0);
        let mut e = engine(Some(balalaika()), 440.0);
        let st = run_frames(&mut e, e4, 1);
        assert_eq!(st.note_midi, Some(64));
        assert_eq!(
            st.string_indices,
            vec![0, 1],
            "пара E4+E4 подсвечивается целиком"
        );
        assert!((st.target_hz.unwrap() - e4 as f32).abs() < 0.05);
        // A4 — третья струна, пара не подтягивается.
        let st = run_frames(&mut e, note::hz(69, 440.0), 25);
        assert_eq!(st.string_indices, vec![2]);
    }

    #[test]
    fn chromatic_nearest_note_without_strings() {
        let mut e = engine(None, 440.0);
        let st = run_frames(&mut e, 443.0, 1);
        assert_eq!(st.kind, TunerKind::Tracking);
        assert_eq!(st.note_midi, Some(69));
        assert_eq!(st.string_indices, Vec::<usize>::new());
        let c = st.cents.unwrap();
        let expected = 1200.0f64 * (443.0f64 / 440.0).log2();
        assert!((f64::from(c) - expected).abs() < 1.0, "cents = {c}");
        assert!((st.target_hz.unwrap() - 440.0).abs() < 0.01);

        let st = run_frames(&mut e, 466.16, 25); // A#4
        assert_eq!(st.note_midi, Some(70));
        assert!(st.cents.unwrap().abs() < 1.0);
    }

    #[test]
    fn a4_442_recomputes_targets() {
        let a2_442 = note::hz(45, 442.0); // ровно 110,5 Гц
        let mut e = engine(Some(guitar()), 442.0);
        let st = run_frames(&mut e, a2_442, 1);
        assert_eq!(st.string_indices, vec![1]);
        assert!((st.target_hz.unwrap() - 110.5).abs() < 0.01);
        assert!(st.cents.unwrap().abs() < 1.0);

        // Смена A4 на 440: цель 110,0 — сглаженная история не сбрасывается (4.3.5),
        // центы съезжают на +7,85, струна остаётся выбранной.
        e.set_config(config(Some(guitar()), 440.0));
        let st = run_frames(&mut e, a2_442, 25);
        assert_eq!(st.string_indices, vec![1]);
        assert!((st.target_hz.unwrap() - 110.0).abs() < 0.01);
        let expected = 1200.0f64 * (110.5f64 / 110.0).log2();
        assert!(
            (f64::from(st.cents.unwrap()) - expected).abs() < 1.0,
            "cents = {}",
            st.cents.unwrap()
        );
    }

    #[test]
    fn tuning_change_resets_smoothing() {
        let mut e = engine(Some(guitar()), 440.0);
        run_frames(&mut e, note::hz(40, 440.0), 25);
        // Новый строй: фильтр сброшен — первый кадр проходит без «приползания».
        e.set_config(config(Some(balalaika()), 440.0));
        let st = run_frames(&mut e, note::hz(64, 440.0), 1);
        assert_eq!(st.note_midi, Some(64));
        assert!(st.cents.unwrap().abs() < 1.0, "после сброса сглаживания");
    }

    #[test]
    fn manual_string_and_foreign_note() {
        let mut matcher = StringMatcher::default();
        let cfg = EngineConfig {
            tuning: Some(guitar()),
            a4: 440.0,
            tolerance_cents: 3.0,
            sensitivity: Sensitivity::Normal,
            manual_string: Some(1),
            low_range: false,
        };
        // A2 точно — выбранная струна.
        let o = matcher.match_f0(note::hz(45, 440.0), &cfg);
        assert_eq!(o.string_indices, vec![1]);
        assert_eq!(o.note_midi, 45);
        assert!(o.cents.abs() < 1e-6);
        assert!(!o.foreign);

        // E2 для выбранной A2 — чужая нота: пара пустая (4.7).
        let o = matcher.match_f0(note::hz(40, 440.0), &cfg);
        assert!(o.foreign);
        assert!(o.string_indices.is_empty());

        // Ручной выбор унисон-пары: любая из E4 даёт пару (6.3.6).
        let cfg_b = EngineConfig {
            tuning: Some(balalaika()),
            a4: 440.0,
            tolerance_cents: 3.0,
            sensitivity: Sensitivity::Normal,
            manual_string: Some(1),
            low_range: false,
        };
        let o = matcher.match_f0(note::hz(64, 440.0), &cfg_b);
        assert_eq!(o.string_indices, vec![0, 1]);
        assert!(!o.foreign);

        // Индекс вне строя — автоопределение, не паника.
        let cfg_bad = EngineConfig {
            manual_string: Some(9),
            ..cfg_b.clone()
        };
        let o = matcher.match_f0(note::hz(69, 440.0), &cfg_bad);
        assert_eq!(o.string_indices, vec![2]);
        assert!(!o.foreign);
    }

    #[test]
    fn manual_string_in_engine_is_uncertain_on_foreign() {
        let cfg = EngineConfig {
            tuning: Some(guitar()),
            a4: 440.0,
            tolerance_cents: 3.0,
            sensitivity: Sensitivity::Normal,
            manual_string: Some(0), // E2
            low_range: false,
        };
        let mut e = TunerEngine::new(cfg, FS);
        // Сначала точно E2 — tracking на выбранной струне.
        let st = run_frames(&mut e, note::hz(40, 440.0), 1);
        assert_eq!(st.kind, TunerKind::Tracking);
        assert_eq!(st.string_indices, vec![0]);
        // Потом чужая A2: kind = uncertain, пара не подсвечивается.
        let st = run_frames(&mut e, note::hz(45, 440.0), 1);
        assert_eq!(st.kind, TunerKind::Uncertain);
        assert!(st.string_indices.is_empty());
        // Последнее показание остаётся (игла на месте, нота приглушена).
        assert_eq!(st.note_midi, Some(40));
        assert!(st.cents.is_some());
    }

    #[test]
    fn sensitivity_shifts_clarity() {
        assert_eq!(Sensitivity::Low.clarity_min(), 0.75);
        assert_eq!(Sensitivity::Normal.clarity_min(), 0.60);
        assert_eq!(Sensitivity::High.clarity_min(), 0.45);
        // Порог тишины фиксирован §4.2.1 и от чувствительности не зависит.
        for sens in [Sensitivity::Low, Sensitivity::Normal, Sensitivity::High] {
            let cfg = detector_config(
                &EngineConfig {
                    sensitivity: sens,
                    ..config(Some(guitar()), 440.0)
                },
                FS,
            );
            assert_eq!(cfg.clarity_min, sens.clarity_min());
            assert_eq!(cfg.gate_db, GATE_DB);
        }
    }

    /// Детерминированный шум (LCG, как в тестах dsp/pitch.rs).
    fn lcg(state: &mut u32) -> f32 {
        *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (*state as f32 / u32::MAX as f32 - 0.5) * 2.0
    }

    /// Синус + шум: достоверность NSDF регулируется амплитудой шума.
    fn noisy_frame(freq: f64, sine_amp: f32, noise_amp: f32) -> Vec<f32> {
        let mut state = 0x5EED_u32;
        (0..4096)
            .map(|i| {
                let t = i as f64 / f64::from(FS);
                sine_amp * (2.0 * std::f64::consts::PI * freq * t).sin() as f32
                    + noise_amp * lcg(&mut state)
            })
            .collect()
    }

    /// Прямое измерение clarity кадра детектором с clarity_min = 0:
    /// обе ветви Estimate несут фактическую достоверность.
    fn measure_clarity(frame: &mut [f32]) -> f32 {
        let mut cfg = DetectorConfig::new(4096, FS, 82.41);
        cfg.clarity_min = 0.0;
        let mut probe = PitchDetector::new(cfg);
        match probe.process_frame(frame, FS) {
            Some(Estimate::Pitch { clarity, .. }) | Some(Estimate::Uncertain { clarity }) => {
                clarity
            }
            _ => 0.0,
        }
    }

    #[test]
    fn mid_clarity_frame_tracks_on_high_and_uncertain_on_normal() {
        // Ищем детерминированный шум с clarity в середине (0.45, 0.60):
        // синтетика вместо живого «играйте чище» (0.3).
        let f = note::hz(40, 440.0);
        let mut chosen = None;
        for step in 0..60 {
            let noise_amp = 0.08 + step as f32 * 0.01;
            let clarity = measure_clarity(&mut noisy_frame(f, 0.12, noise_amp));
            if (0.48..0.57).contains(&clarity) {
                chosen = Some((noise_amp, clarity));
                break;
            }
        }
        let Some((noise_amp, clarity)) = chosen else {
            panic!("не нашли амплитуду шума с clarity в (0.45, 0.60)");
        };

        // High: clarity ≥ 0.45 → Pitch → Tracking.
        let mut high = TunerEngine::new(
            EngineConfig {
                sensitivity: Sensitivity::High,
                ..config(Some(guitar()), 440.0)
            },
            FS,
        );
        let mut frame = noisy_frame(f, 0.12, noise_amp);
        let st = high.process_frame(&mut frame, FS);
        assert_eq!(st.kind, TunerKind::Tracking, "clarity = {clarity}");

        // Normal: тот же кадр, clarity < 0.60 → Uncertain.
        let mut normal = engine(Some(guitar()), 440.0);
        let mut frame = noisy_frame(f, 0.12, noise_amp);
        let st = normal.process_frame(&mut frame, FS);
        assert_eq!(st.kind, TunerKind::Uncertain, "clarity = {clarity}");
    }

    #[test]
    fn tuner_state_serializes_camel_case() {
        let st = TunerState {
            kind: TunerKind::InTune,
            f0: Some(82.41),
            cents: Some(0.0),
            clarity: 0.93,
            db: -23.5,
            note_midi: Some(40),
            target_hz: Some(82.41),
            string_indices: vec![0],
        };
        let v = serde_json::to_value(&st).expect("сериализация");
        assert_eq!(v["kind"], "intune");
        assert!((v["f0"].as_f64().unwrap() - 82.41).abs() < 1e-3);
        assert_eq!(v["noteMidi"], 40);
        assert!((v["targetHz"].as_f64().unwrap() - 82.41).abs() < 1e-3);
        assert_eq!(v["stringIndices"], serde_json::json!([0]));
        assert!(v.get("note_midi").is_none(), "camelCase, не snake_case");
        assert!((v["clarity"].as_f64().unwrap() - 0.93).abs() < 1e-3);
        assert_eq!(v["db"], -23.5);
    }

    #[test]
    fn window_follows_tuning() {
        let mut e = engine(Some(guitar()), 440.0);
        assert_eq!(e.window(), 4096);
        let bass5 = InstrumentRegistry::new()
            .tuning("bass", "bass5")
            .unwrap()
            .clone();
        e.set_config(config(Some(bass5), 440.0));
        assert_eq!(e.window(), 16384);
        assert_eq!(engine(None, 440.0).window(), 4096);
    }

    #[test]
    fn low_range_switches_chromatic_window() {
        // «Низкий диапазон» (5.3): окно хроматики 4096 → 16384 (4.2.3, D-3).
        let mut e = engine(None, 440.0);
        assert_eq!(e.window(), 4096);
        e.set_config(EngineConfig {
            low_range: true,
            ..config(None, 440.0)
        });
        assert_eq!(e.window(), 16384);
        // Выключение возвращает базовое окно.
        e.set_config(config(None, 440.0));
        assert_eq!(e.window(), 4096);
    }
}
