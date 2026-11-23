//! Заглушка `AudioSource` для Android (этап 5, docs/source/07_platform.md).
//!
//! Нужна, чтобы APK собирался и CI был зелёным: `bridge` требует
//! `PlatformSource`, а настоящий бэкенд — `oboe` (7.1.2) — приедет на
//! этапе 5. UI на Android получает ошибку устройства и показывает экран
//! «Повторить» — это ожидаемо для текущего этапа.

use super::{AudioError, AudioSource};

/// Заглушка: захват звука на Android — этап 5 (oboe).
pub struct StubSource {
    sample_rate: u32,
}

impl StubSource {
    pub fn new() -> Self {
        Self { sample_rate: 0 }
    }
}

impl Default for StubSource {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioSource for StubSource {
    /// Номинально 48 кГц; до успешного `start` (который здесь не бывает) — 0.
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn start(&mut self, _sink: rtrb::Producer<f32>) -> Result<(), AudioError> {
        Err(AudioError::Device(
            "захват звука на Android — этап 5".into(),
        ))
    }

    fn stop(&mut self) {
        self.sample_rate = 0;
    }
}
