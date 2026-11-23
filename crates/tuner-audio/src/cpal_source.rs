//! `CpalSource`: захват через `cpal` → CoreAudio (7.2.1), WASAPI (7.2.6),
//! ALSA/PipeWire (7.2.7).
//!
//! Конфигурация: моно, `f32`, 48 кГц (fallback — фактическая конфигурация
//! устройства), буфер 512 — запрос, а не гарантия. Реал-тайм-колбэк пишет
//! отсчёты в `rtrb::Producer` и считает переполнения; ничего не аллоцирует,
//! не блокируется и не логирует (3.6). Первый `stream.play()` вызывает
//! системный запрос доступа к микрофону (TCC, 7.2.1); при отказе macOS может
//! отдавать тишину — отдельного TCC-статуса у cpal нет.
//!
//! `cpal::Stream` на macOS не `Send`, поэтому живёт в отдельном
//! **потоке-владельце**: открытие/закрытие устройства (десятки мс, 7.1.2)
//! происходит вне командного потока, а сам `Stream` не пересекает границы
//! потоков — без `unsafe`.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::Duration;

use super::{AudioError, AudioSource};

/// Размер буфера устройства (кадры): запрос через `BufferSize::Fixed` (7.2.1).
const BUFFER_SIZE: u32 = 512;

/// Команды потоку-владельцу устройства.
enum Command {
    /// Открыть устройство; ответ — фактическая частота дискретизации.
    Start {
        producer: rtrb::Producer<f32>,
        reply: mpsc::Sender<Result<u32, AudioError>>,
    },
    /// Остановить поток и освободить устройство.
    Stop { reply: mpsc::Sender<()> },
}

/// Поток-владелец: держит `cpal::Stream`, пока не получит Stop
/// (или пока `CpalSource` не дропнется — тогда канал закрывается и
/// `Stream` падает вместе с потоком: захват гарантированно останавливается).
struct Owner {
    tx: mpsc::Sender<Command>,
    thread: JoinHandle<()>,
}

impl Owner {
    fn spawn(overruns: Arc<AtomicU32>) -> Self {
        let (tx, rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("unisono-cpal-owner".into())
            .spawn(move || owner_loop(rx, overruns))
            .expect("не удалось создать поток-владелец");
        Self { tx, thread }
    }
}

fn owner_loop(rx: mpsc::Receiver<Command>, overruns: Arc<AtomicU32>) {
    // Stream живёт, пока лежит в этой переменной; take()+drop() освобождает
    // устройство (7.1.4).
    let mut stream: Option<cpal::Stream> = None;
    while let Ok(cmd) = rx.recv() {
        match cmd {
            Command::Start { producer, reply } => {
                drop(stream.take()); // повторный start переоткрывает устройство
                match open_stream(producer, Arc::clone(&overruns)) {
                    Ok((s, rate)) => {
                        stream = Some(s);
                        let _ = reply.send(Ok(rate));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            Command::Stop { reply } => {
                drop(stream.take());
                let _ = reply.send(());
            }
        }
    }
    // Канал закрыт (CpalSource дропнут): Stream падает здесь, захват стоп.
}

/// Захват через `cpal`. Ошибки устройства — значения [`AudioError`];
/// переполнения ring buffer считаются атомарно и не паникуют.
pub struct CpalSource {
    sample_rate: u32,
    overruns: Arc<AtomicU32>,
    owner: Option<Owner>,
}

impl CpalSource {
    pub fn new() -> Self {
        Self {
            sample_rate: 0,
            overruns: Arc::new(AtomicU32::new(0)),
            owner: None,
        }
    }

    /// Переполнения ring buffer с последнего `start` (диагностика пропусков).
    pub fn overruns(&self) -> u32 {
        self.overruns.load(Ordering::Relaxed)
    }
}

impl Default for CpalSource {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioSource for CpalSource {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn start(&mut self, sink: rtrb::Producer<f32>) -> Result<(), AudioError> {
        self.stop();
        self.overruns = Arc::new(AtomicU32::new(0));
        let owner = Owner::spawn(Arc::clone(&self.overruns));
        let (tx_reply, rx_reply) = mpsc::channel();
        owner
            .tx
            .send(Command::Start {
                producer: sink,
                reply: tx_reply,
            })
            .map_err(|_| AudioError::Stream("поток-владелец недоступен".into()))?;
        // Открытие устройства — десятки мс (7.1.2); ждём ответ здесь,
        // командный поток не аудио и не UI.
        match rx_reply.recv() {
            Ok(Ok(rate)) => {
                self.sample_rate = rate;
                self.owner = Some(owner);
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(AudioError::Stream("поток-владелец умер".into())),
        }
    }

    fn stop(&mut self) {
        if let Some(owner) = self.owner.take() {
            let (tx_reply, rx_reply) = mpsc::channel();
            if owner.tx.send(Command::Stop { reply: tx_reply }).is_ok() {
                // Устройство гарантированно освобождено после ответа.
                let _ = rx_reply.recv_timeout(Duration::from_secs(5));
            }
            let _ = owner.thread.join();
        }
        self.sample_rate = 0;
    }
}

/// Открытие устройства: моно f32 @ 48 кГц приоритетно, fallback — фактическая
/// конфигурация (7.2.1); частота берётся из устройства, не из константы.
fn open_stream(
    producer: rtrb::Producer<f32>,
    overruns: Arc<AtomicU32>,
) -> Result<(cpal::Stream, u32), AudioError> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(AudioError::NoInputDevice)?;
    let supported = device
        .default_input_config()
        .map_err(|e| config_error(&e.to_string()))?;

    let (config, channels) = match device.supported_input_configs() {
        Ok(ranges) => {
            let mono_48k = ranges.into_iter().any(|r| {
                r.channels() == 1
                    && r.sample_format() == cpal::SampleFormat::F32
                    && r.min_sample_rate().0 <= 48_000
                    && r.max_sample_rate().0 >= 48_000
            });
            if mono_48k {
                (
                    cpal::StreamConfig {
                        channels: 1,
                        sample_rate: cpal::SampleRate(48_000),
                        buffer_size: cpal::BufferSize::Fixed(BUFFER_SIZE),
                    },
                    1,
                )
            } else {
                (
                    cpal::StreamConfig {
                        channels: supported.channels(),
                        sample_rate: supported.sample_rate(),
                        buffer_size: cpal::BufferSize::Fixed(BUFFER_SIZE),
                    },
                    supported.channels() as usize,
                )
            }
        }
        // Список конфигураций недоступен — пробуем предпочитаемую.
        Err(_) => (
            cpal::StreamConfig {
                channels: 1,
                sample_rate: cpal::SampleRate(48_000),
                buffer_size: cpal::BufferSize::Fixed(BUFFER_SIZE),
            },
            1,
        ),
    };

    let stream = build_stream(&device, &config, channels, producer, overruns)?;
    // Первый play() поднимает системный запрос микрофона (TCC, 7.2.1).
    stream
        .play()
        .map_err(|e| AudioError::Stream(e.to_string()))?;
    Ok((stream, config.sample_rate.0))
}

/// Сборка входного потока: колбэк без аллокаций — только `Producer` и счётчик.
fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    mut producer: rtrb::Producer<f32>,
    overruns: Arc<AtomicU32>,
) -> Result<cpal::Stream, AudioError> {
    use cpal::traits::DeviceTrait;
    let data_cb = move |data: &[f32], _info: &cpal::InputCallbackInfo| {
        // Реал-тайм (3.6): моно-канал из interleaved, без аллокаций и блокировок.
        for &sample in data.iter().step_by(channels.max(1)) {
            // Переполнение не должно ронять аудио-поток — только счётчик.
            if producer.push(sample).is_err() {
                overruns.fetch_add(1, Ordering::Relaxed);
            }
        }
    };
    let err_cb = |err: cpal::StreamError| tracing::error!(?err, "audio stream error");
    device
        .build_input_stream(config, data_cb, err_cb, None)
        .map_err(|e| stream_error(&e.to_string()))
}

/// Ошибка конфигурации устройства: TCC-отказ распознаём по описанию (7.2).
fn config_error(msg: &str) -> AudioError {
    if looks_like_permission_denied(msg) {
        AudioError::PermissionDenied
    } else {
        AudioError::Device(msg.to_string())
    }
}

fn stream_error(msg: &str) -> AudioError {
    if looks_like_permission_denied(msg) {
        AudioError::PermissionDenied
    } else {
        AudioError::Stream(msg.to_string())
    }
}

/// macOS не даёт отдельного кода ошибки TCC; распознаём по описанию.
fn looks_like_permission_denied(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    m.contains("permission")
        || m.contains("not authorized")
        || m.contains("denied")
        || m.contains("tcc")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_source_is_stopped_and_empty() {
        let mut src = CpalSource::new();
        assert_eq!(src.sample_rate(), 0);
        assert_eq!(src.overruns(), 0);
        // stop без запуска — не паника.
        src.stop();
        assert_eq!(src.sample_rate(), 0);
    }

    #[test]
    fn permission_errors_are_recognized() {
        assert!(looks_like_permission_denied(
            "operation not permitted by TCC"
        ));
        assert!(looks_like_permission_denied("Permission denied"));
        assert!(!looks_like_permission_denied("invalid sample format"));
        assert!(matches!(
            config_error("not authorized"),
            AudioError::PermissionDenied
        ));
        assert!(matches!(
            stream_error("unsupported buffer size"),
            AudioError::Stream(_)
        ));
    }

    /// Дроп source без stop() тоже останавливает захват: канал владельца
    /// закрывается, Stream падает в потоке-владельце.
    #[test]
    fn drop_without_stop_is_sound() {
        let src = CpalSource::new();
        drop(src);
    }
}
