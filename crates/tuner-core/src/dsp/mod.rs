//! DSP: определение высоты тона NSDF/MPM (docs/source/04_dsp.md).
//!
//! Чистый Rust, `f32` в горячем цикле, без I/O, без платформенного кода,
//! без аллокаций в `process_frame`. Окно анализа приходит из реестра
//! инструментов (`music::InstrumentRegistry`), а не фиксируется глобально.

pub mod gate;
pub mod nsdf;
pub mod pitch;
pub mod smooth;
pub mod window;

pub use gate::{level_db, GateState, LevelGate, GATE_DB, GATE_HYSTERESIS_DB};
pub use nsdf::{energy_prefix, nsdf};
pub use pitch::{DetectorConfig, Estimate, PitchDetector};
pub use smooth::SmoothingFilter;
pub use window::{tau_max, tau_min, F_MAX_HZ, F_MIN_HZ, WINDOWS};
