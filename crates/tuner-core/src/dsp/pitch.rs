//! Детектор высоты тона: NSDF/MPM (docs/source/04_dsp.md, 4.3).
//!
//! Конвейер кадра: DC-remove → ФВЧ → RMS-гейт → NSDF → первый пик ≥ κ·max →
//! параболическая интерполяция → октавная коррекция → clarity.
//! Все буферы создаются в `new`; `process_frame` не аллоцирует.

use super::gate::{level_db, GateState, LevelGate};
use super::nsdf::{energy_prefix, nsdf};
use super::window::{tau_max, tau_min, F_MIN_HZ};
use crate::music::{note, InstrumentRegistry, Tuning};

/// Параметры детектора (4.2.1). Окно и границы τ — от фактического строя и fs.
#[derive(Clone, Copy, Debug)]
pub struct DetectorConfig {
    /// Окно анализа: 4096 / 8192 / 16384.
    pub window: usize,
    pub tau_min: usize,
    pub tau_max: usize,
    /// Порог выхода из тишины, dBFS (вход = gate_db − 5).
    pub gate_db: f32,
    /// Минимальная достоверность для показа цифры.
    pub clarity_min: f32,
    /// Порог правила McLeod: первый пик ≥ κ·n_max.
    pub kappa: f32,
    /// Нижняя целевая частота (Гц) при текущей A4: задаёт τ_max и ФВЧ.
    pub f_min_hz: f32,
}

impl DetectorConfig {
    /// Конфиг для окна и нижней целевой частоты; пороги — по 4.2.1.
    pub fn new(window: usize, fs: u32, f_min_hz: f32) -> Self {
        Self {
            window,
            tau_min: tau_min(fs),
            tau_max: tau_max(fs, f_min_hz, window),
            gate_db: super::gate::GATE_DB,
            clarity_min: 0.60,
            kappa: 0.90,
            f_min_hz,
        }
    }

    /// Конфиг по активному строю: окно из `InstrumentRegistry::window_size`,
    /// `f_min` — фактическая нижняя цель при текущей опорной A4.
    pub fn for_tuning(tuning: &Tuning, a4: f64, fs: u32) -> Self {
        let window = InstrumentRegistry::window_size(tuning) as usize;
        let f_min = InstrumentRegistry::lowest_note(tuning)
            .map(|m| note::hz(m, a4) as f32)
            .unwrap_or(F_MIN_HZ);
        Self::new(window, fs, f_min)
    }

    /// Конфиг хроматического режима: 4096, с «Низким диапазоном» — 16384.
    pub fn for_chromatic(low_range: bool, fs: u32) -> Self {
        let window = InstrumentRegistry::chromatic_window(low_range) as usize;
        Self::new(window, fs, F_MIN_HZ)
    }
}

/// Результат обработки кадра (4.3.7, 4.4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Estimate {
    /// Полоса гистерезиса: «Слушаю…», без цифр.
    Silent,
    /// Сигнал есть, но достоверность ниже порога.
    Uncertain { clarity: f32 },
    /// Оценка основного тона.
    Pitch { f0: f32, clarity: f32, db: f32 },
}

/// Детектор основного тона. Буферы — поля, создаются один раз в `new`.
pub struct PitchDetector {
    config: DetectorConfig,
    /// NSDF, индексируется τ; длина window/2 + 2.
    nsdf: Vec<f32>,
    /// Префиксные суммы энергий кадра; длина window + 1.
    prefix: Vec<f64>,
    gate: LevelGate,
    /// Состояние one-pole ФВЧ между кадрами.
    hp_y: f32,
    hp_x: f32,
}

impl PitchDetector {
    pub fn new(config: DetectorConfig) -> Self {
        Self {
            nsdf: vec![0.0; config.window / 2 + 2],
            prefix: vec![0.0; config.window + 1],
            gate: LevelGate {
                gate_db: config.gate_db,
                hysteresis_db: super::gate::GATE_HYSTERESIS_DB,
            },
            config,
            hp_y: 0.0,
            hp_x: 0.0,
        }
    }

    pub fn config(&self) -> &DetectorConfig {
        &self.config
    }

    /// Обрабатывает кадр длины `config.window`: без аллокаций, in-place.
    ///
    /// `None` — тишина ниже нижнего порога; кадр короче окна не анализируется.
    pub fn process_frame(&mut self, raw: &mut [f32], fs: u32) -> Option<Estimate> {
        let n = self.config.window;
        if raw.len() < n {
            return None;
        }
        let frame = &mut raw[..n];

        // 1. Удаление постоянной составляющей (иначе ложный пик на больших τ).
        let mean: f32 = frame.iter().sum::<f32>() / n as f32;
        for x in frame.iter_mut() {
            *x -= mean;
        }

        // 2. One-pole ФВЧ: fc = max(40 Гц, 0.6 · f_min), a = 1/(1 + 2π·fc/fs).
        let fc = 40.0f32.max(0.6 * self.config.f_min_hz);
        let a = 1.0 / (1.0 + 2.0 * std::f32::consts::PI * fc / fs as f32);
        let mut y = self.hp_y;
        let mut x_prev = self.hp_x;
        for x in frame.iter_mut() {
            let xi = *x;
            y = a * (y + xi - x_prev);
            x_prev = xi;
            *x = y;
        }
        self.hp_y = y;
        self.hp_x = x_prev;

        // 3. Уровень и гейт с гистерезисом.
        let db = level_db(frame);
        match self.gate.check(db) {
            GateState::BelowSilence => return None,
            GateState::InHysteresis => return Some(Estimate::Silent),
            GateState::Open => {}
        }

        // 4. NSDF.
        energy_prefix(frame, &mut self.prefix);
        nsdf(
            frame,
            &self.prefix,
            self.config.tau_min,
            self.config.tau_max,
            &mut self.nsdf,
        );

        // 5. Правило McLeod: наименьший локальный максимум ≥ κ·n_max.
        let peak = self.first_peak_above(self.config.kappa)?;
        let tau = self.parabolic_refine(peak);
        let mut f0 = fs as f32 / tau;
        let mut clarity = self.nsdf[peak].min(1.0);

        // 6. Октавная коррекция (4.3.4): перегруз и низкие струны.
        if clarity < 0.75 {
            let t2 = 2.0 * tau;
            let c2 = self.clarity_at(t2);
            if t2 <= self.config.tau_max as f32 && c2 >= 0.85 {
                f0 = fs as f32 / t2;
                clarity = c2;
            } else {
                let th = tau / 2.0;
                let ch = self.clarity_at(th);
                if th >= self.config.tau_min as f32 && ch >= 0.90 {
                    f0 = fs as f32 / th;
                    clarity = ch;
                }
            }
        }

        if clarity < self.config.clarity_min {
            return Some(Estimate::Uncertain { clarity });
        }
        Some(Estimate::Pitch { f0, clarity, db })
    }

    /// Локальные максимумы NSDF; первый (наименьший τ) с `n ≥ kappa · n_max`.
    fn first_peak_above(&self, kappa: f32) -> Option<usize> {
        let lo = self.config.tau_min + 1;
        let hi = self.config.tau_max;
        let mut n_max = 0f32;
        for t in lo..hi {
            let v = self.nsdf[t];
            if v > self.nsdf[t - 1] && v >= self.nsdf[t + 1] && v > n_max {
                n_max = v;
            }
        }
        if n_max <= 0.0 {
            return None;
        }
        let threshold = kappa * n_max;
        (lo..hi).find(|&t| {
            let v = self.nsdf[t];
            v > self.nsdf[t - 1] && v >= self.nsdf[t + 1] && v >= threshold
        })
    }

    /// Параболическая интерполяция вершины пика (4.3.4), f64.
    fn parabolic_refine(&self, k: usize) -> f32 {
        let y0 = f64::from(self.nsdf[k - 1]);
        let y1 = f64::from(self.nsdf[k]);
        let y2 = f64::from(self.nsdf[k + 1]);
        let den = y0 - 2.0 * y1 + y2;
        let delta = if den.abs() < 1e-12 {
            0.0
        } else {
            ((y0 - y2) / (2.0 * den)).clamp(-1.0, 1.0)
        };
        (k as f64 + delta) as f32
    }

    /// Достоверность кандидата на произвольном τ (для октавной коррекции).
    fn clarity_at(&self, tau: f32) -> f32 {
        let k = tau.round() as usize;
        if k < self.config.tau_min + 1 || k > self.config.tau_max {
            return 0.0;
        }
        self.nsdf[k].min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::InstrumentRegistry;

    const FS: u32 = 48_000;

    fn sine(freq: f32, n: usize, amp: f32) -> Vec<f32> {
        (0..n)
            .map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / FS as f32).sin())
            .collect()
    }

    fn detector(window: usize, f_min: f32) -> PitchDetector {
        PitchDetector::new(DetectorConfig::new(window, FS, f_min))
    }

    #[test]
    fn pure_sine_440() {
        let mut d = detector(4096, 82.41);
        let mut frame = sine(440.0, 4096, 0.1);
        let est = d.process_frame(&mut frame, FS).unwrap();
        match est {
            Estimate::Pitch { f0, clarity, .. } => {
                let cents = 1200.0 * (f64::from(f0) / 440.0).log2();
                assert!(cents.abs() < 0.5, "cents = {cents}");
                assert!(clarity > 0.9);
            }
            other => panic!("ожидался Pitch, получен {other:?}"),
        }
    }

    #[test]
    fn low_sine_b0_with_big_window() {
        let mut d = detector(16384, 30.87);
        let mut frame = sine(30.87, 16384, 0.1);
        match d.process_frame(&mut frame, FS).unwrap() {
            Estimate::Pitch { f0, .. } => {
                let cents = 1200.0 * (f64::from(f0) / 30.87).log2();
                assert!(cents.abs() < 3.0, "cents = {cents}");
            }
            other => panic!("ожидался Pitch, получен {other:?}"),
        }
    }

    #[test]
    fn silence_and_hysteresis() {
        let mut d = detector(4096, 82.41);
        let mut quiet = vec![0f32; 4096];
        assert_eq!(d.process_frame(&mut quiet, FS), None);
        // −47 dBFS: полоса гистерезиса → Silent.
        let amp = 10f32.powf(-47.0 / 20.0) * 2f32.sqrt();
        let mut band = sine(440.0, 4096, amp);
        assert_eq!(d.process_frame(&mut band, FS), Some(Estimate::Silent));
    }

    #[test]
    fn missing_fundamental_picks_true_period() {
        // Гармоники 2 и 3 от 220 Гц: основного тона нет, период всё равно 220.
        let mut d = detector(4096, 82.41);
        let mut frame: Vec<f32> = (0..4096)
            .map(|i| {
                let t = i as f32 / FS as f32;
                0.5 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()
                    + 0.5 * (2.0 * std::f32::consts::PI * 660.0 * t).sin()
            })
            .collect();
        match d.process_frame(&mut frame, FS).unwrap() {
            Estimate::Pitch { f0, .. } => {
                let cents = 1200.0 * (f64::from(f0) / 220.0).log2();
                assert!(cents.abs() < 1.0, "cents = {cents}");
            }
            other => panic!("ожидался Pitch 220 Гц, получен {other:?}"),
        }
    }

    #[test]
    fn noise_is_not_a_note() {
        let mut d = detector(4096, 82.41);
        let mut state = 12345u32;
        let mut frame: Vec<f32> = (0..4096)
            .map(|_| {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                (state as f32 / u32::MAX as f32 - 0.5) * 0.2
            })
            .collect();
        let est = d.process_frame(&mut frame, FS);
        assert!(
            !matches!(est, Some(Estimate::Pitch { .. })),
            "шум не должен давать ноту: {est:?}"
        );
    }

    #[test]
    fn config_from_tuning_uses_registry_window() {
        let reg = InstrumentRegistry::new();
        let guitar = reg.tuning("guitar", "standard_e").unwrap();
        let c = DetectorConfig::for_tuning(guitar, 440.0, FS);
        assert_eq!(c.window, 4096);
        assert_eq!(c.tau_max, 619);
        let bass5 = reg.tuning("bass", "bass5").unwrap();
        let c = DetectorConfig::for_tuning(bass5, 440.0, FS);
        assert_eq!(c.window, 16384);
        // При A4 = 400 нижняя цель ниже → τ_max больше.
        let c400 = DetectorConfig::for_tuning(bass5, 400.0, FS);
        assert!(c400.tau_max > c.tau_max);
        let chrom = DetectorConfig::for_chromatic(false, FS);
        assert_eq!(chrom.window, 4096);
        assert_eq!(DetectorConfig::for_chromatic(true, FS).window, 16384);
    }

    #[test]
    fn short_frame_is_not_analyzed() {
        let mut d = detector(4096, 82.41);
        let mut frame = sine(440.0, 1000, 0.1);
        assert_eq!(d.process_frame(&mut frame, FS), None);
    }
}
