//! RMS-гейт с гистерезисом тишины (docs/source/04_dsp.md, 4.2.1, 4.3.1).
//!
//! Вход в тишину: RMS < −50 dBFS; выход: RMS > −45 dBFS. Полоса 5 дБ между
//! порогами не даёт индикатору мигать на затухающей струне.

/// Порог выхода из тишины (dBFS).
pub const GATE_DB: f32 = -45.0;
/// Гистерезис: порог входа в тишину = GATE_DB − 5 дБ.
pub const GATE_HYSTERESIS_DB: f32 = 5.0;

/// Решение гейта по уровню кадра.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateState {
    /// Ниже −50 dBFS: кадра нет, анализ не выполняется (`None`).
    BelowSilence,
    /// Полоса −50…−45 dBFS: состояние «Слушаю…» без цифр (`Estimate::Silent`).
    InHysteresis,
    /// Выше −45 dBFS: сигнал есть, выполняем анализ.
    Open,
}

/// RMS-гейт с двумя порогами (гистерезис 5 дБ).
#[derive(Clone, Copy, Debug)]
pub struct LevelGate {
    pub gate_db: f32,
    pub hysteresis_db: f32,
}

impl Default for LevelGate {
    fn default() -> Self {
        Self::new()
    }
}

impl LevelGate {
    pub fn new() -> Self {
        Self {
            gate_db: GATE_DB,
            hysteresis_db: GATE_HYSTERESIS_DB,
        }
    }

    pub fn check(&self, db: f32) -> GateState {
        if db < self.gate_db - self.hysteresis_db {
            GateState::BelowSilence
        } else if db < self.gate_db {
            GateState::InHysteresis
        } else {
            GateState::Open
        }
    }
}

/// Уровень кадра в dBFS: `20 · log10(rms + 1e-9)` (4.3.1).
pub fn level_db(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return -180.0;
    }
    let sum: f64 = frame.iter().map(|&x| f64::from(x) * f64::from(x)).sum();
    let rms = (sum / frame.len() as f64).sqrt();
    (20.0 * (rms + 1e-9).log10()) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_and_hysteresis_band() {
        let gate = LevelGate::new();
        assert_eq!(gate.check(-60.0), GateState::BelowSilence);
        assert_eq!(gate.check(-50.1), GateState::BelowSilence);
        assert_eq!(gate.check(-50.0), GateState::InHysteresis);
        assert_eq!(gate.check(-47.0), GateState::InHysteresis);
        assert_eq!(gate.check(-45.0), GateState::Open);
        assert_eq!(gate.check(-20.0), GateState::Open);
    }

    #[test]
    fn level_db_of_known_signals() {
        // Синус амплитуды 0.1: rms = 0.1/√2 ≈ 0.0707 → −23 dBFS.
        let sine: Vec<f32> = (0..4800)
            .map(|i| 0.1 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
            .collect();
        let db = level_db(&sine);
        assert!((db - (-23.0)).abs() < 0.2, "db = {db}");
        // Тишина −60 dBFS: rms = 1e-3.
        let quiet = vec![1e-3f32; 4800];
        assert!((level_db(&quiet) - (-60.0)).abs() < 0.1);
        assert!(level_db(&[]) < -100.0);
    }
}
