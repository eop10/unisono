//! Захват звука на Android через oboe / AAudio (docs/source/07_platform.md §7.1.2).
//!
//! Real-time колбэк без аллокаций, мьютексов и системных вызовов: сэмплы `f32`
//! уходят в `rtrb::Producer`. Открытие потока через `AudioStreamBuilder` с
//! `AudioFormat::Float`, `Mono`, `InputPreset::VoiceRecognition` (без AGC/NS).

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use ::oboe::{
    AudioInputCallback, AudioInputStreamSafe, AudioStream, AudioStreamAsync, AudioStreamBase,
    AudioStreamBuilder, DataCallbackResult, Error as OboeError, Input, InputPreset, Mono,
};

use super::{AudioError, AudioSource};

/// Колбэк аудио: выполняется в real-time потоке AAudio/OpenSLES (3.6).
/// Без аллокаций, блокировок и форматирования строк.
struct Capture {
    producer: rtrb::Producer<f32>,
    overruns: Arc<AtomicU32>,
    failed: Arc<AtomicBool>,
}

impl AudioInputCallback for Capture {
    type FrameType = (f32, Mono);

    fn on_audio_ready(
        &mut self,
        _stream: &mut dyn AudioInputStreamSafe,
        data: &[f32],
    ) -> DataCallbackResult {
        for &sample in data {
            if self.producer.push(sample).is_err() {
                self.overruns.fetch_add(1, Ordering::Relaxed);
            }
        }
        DataCallbackResult::Continue
    }

    fn on_error_after_close(&mut self, _stream: &mut dyn AudioInputStreamSafe, _error: OboeError) {
        self.failed.store(true, Ordering::Release);
    }
}

/// Источник звука Android на базе oboe (7.1.2).
pub struct OboeSource {
    sample_rate: u32,
    stream: Option<AudioStreamAsync<Input, Capture>>,
    failed: Arc<AtomicBool>,
}

// SAFETY: `AudioStreamAsync` содержит C++ shared_ptr `AudioStreamShared`,
// который не имеет Send в Rust bindgen. `OboeSource` монопольно владеет потоком
// и управляет им через `&mut self` в потоке-владельце.
unsafe impl Send for OboeSource {}

impl OboeSource {
    pub fn new() -> Self {
        Self {
            sample_rate: 0,
            stream: None,
            failed: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for OboeSource {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioSource for OboeSource {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn start(&mut self, sink: rtrb::Producer<f32>) -> Result<(), AudioError> {
        self.stop();
        self.failed.store(false, Ordering::Release);

        let capture = Capture {
            producer: sink,
            overruns: Arc::new(AtomicU32::new(0)),
            failed: Arc::clone(&self.failed),
        };

        let mut builder = AudioStreamBuilder::default()
            .set_input()
            .set_f32()
            .set_mono();

        builder = builder
            .set_sample_rate(48_000)
            .set_input_preset(InputPreset::VoiceRecognition);

        let mut stream = builder
            .set_callback(capture)
            .open_stream()
            .map_err(map_oboe_error)?;

        stream.start().map_err(map_oboe_error)?;

        let fs = stream.get_sample_rate();
        self.sample_rate = if fs > 0 { fs as u32 } else { 48_000 };
        self.stream = Some(stream);

        Ok(())
    }

    fn stop(&mut self) {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.stop();
            let _ = stream.close();
        }
        self.sample_rate = 0;
    }

    fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
}

fn map_oboe_error(err: OboeError) -> AudioError {
    match err {
        // В AAudio/Oboe отсутствие разрешения RECORD_AUDIO на уровне AudioFlinger
        // возвращает Error::Unavailable или Error::Internal.
        OboeError::Unavailable | OboeError::Internal => AudioError::PermissionDenied,
        other => AudioError::Device(format!("{other:?}")),
    }
}
