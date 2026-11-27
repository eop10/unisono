//! tuner-audio: захват микрофона и передача `f32`-кадров анализатору.
//!
//! Trait `AudioSource` (docs/source/07_platform.md): бэкенд `cpal` — desktop
//! (macOS/WASAPI/ALSA), `oboe` — Android, этап 5. Real-time колбэк без
//! аллокаций и блокировок: только `rtrb::Producer` и счётчик переполнений
//! (docs/source/03_architecture.md, 3.6). Крейт не знает про Tauri и UI:
//! состояния отдаёт колбэк `TunerState`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use tuner_core::engine::{TunerEngine, TunerState, HOP};

/// Ёмкость ring buffer: ~1 с при 48 кГц (3.6).
pub const RING_CAPACITY: usize = 48_000;

/// Ошибки аудио-слоя (3.6, 7.2.1): значения, не паники. Доходят до bridge
/// и дальше в UI как состояния `Error`/`NoPermission`, а не как крах.
/// `Serialize` — тип ошибки команды `start_listening` в Tauri; формат
/// стабильный тегированный объект: `{"kind":"permissionDenied"}`,
/// `{"kind":"device","message":"busy"}`.
#[derive(Debug, thiserror::Error, serde::Serialize, Clone)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum AudioError {
    /// Нет устройства ввода.
    #[error("нет устройства ввода")]
    NoInputDevice,
    /// Разрешение на микрофон не выдано или отозвано (TCC/системные настройки).
    #[error("нет доступа к микрофону")]
    PermissionDenied,
    /// Устройство занято или недоступно.
    #[error("микрофон занят или недоступен: {0}")]
    Device(String),
    /// Поток не удалось запустить.
    #[error("не удалось запустить поток: {0}")]
    Stream(String),
}

/// Источник звука (7): открыть микрофон, отдавать `f32`-кадры в канал.
///
/// Реализация обязана держать реал-тайм-обещание: колбэк не аллоцирует,
/// не блокируется и не логирует (3.6) — только запись в `rtrb::Producer`.
pub trait AudioSource: Send {
    /// Фактическая частота дискретизации, установленная устройством.
    /// Осмысленна после успешного `start`.
    fn sample_rate(&self) -> u32;
    /// Запускает поток; кадры `f32` (моно) уходят в `sink` без аллокаций.
    fn start(&mut self, sink: rtrb::Producer<f32>) -> Result<(), AudioError>;
    /// Останавливает поток и освобождает устройство; `start` открывает заново.
    fn stop(&mut self);
    /// Сообщал ли колбэк потока об ошибке: сигнал наблюдателю переоткрыть
    /// устройство (7.2.1). Реализации без ошибок потока возвращают `false`.
    fn failed(&self) -> bool {
        false
    }
}

/// Запускает поток анализа (3.6): ring buffer → окно движка → `on_state`.
///
/// Поток живёт, пока жив `Producer` (его держит аудио-колбэк). При остановке
/// захвата `Consumer::is_abandoned()` становится true, и поток выходит сам:
/// «окно живёт отдельно от аудио». Окно 4096/8192/16384 перечитывается у
/// движка после каждого кадра — смена строя переключает его на ходу.
///
/// История в `buffer` остаётся **сырой**: `PitchDetector::process_frame`
/// фильтрует кадр in-place (§4.3.7), поэтому детектору отдаётся копия в
/// `scratch` — иначе перекрытие окна фильтровалось бы повторно (до 4 раз
/// для 4096 при hop 1024) и скачки на границах hop портили затухающие струны.
/// Мьютекс движка не удерживается во время `on_state`: состояние и окно
/// снимаются под замком, затем замок освобождается до отправки в канал.
///
/// `engine` — общий с bridge (`Arc<Mutex>`): команды меняют конфиг через
/// `set_config` под тем же замком; контендиции нет (≈47 Гц, без аудио-пути).
pub fn spawn_analyzer(
    mut consumer: rtrb::Consumer<f32>,
    engine: Arc<Mutex<TunerEngine>>,
    fs: u32,
    stop: Arc<AtomicBool>,
    mut on_state: impl FnMut(TunerState) + Send + 'static,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("unisono-analyzer".into())
        .spawn(move || {
            // Сырая история (перекрывающиеся окна) и копия для детектора.
            let mut buffer: Vec<f32> = Vec::with_capacity(16_384);
            let mut scratch: Vec<f32> = Vec::with_capacity(16_384);
            let mut window = engine.lock().map(|e| e.window()).unwrap_or(4096);
            loop {
                // Наполняем буфер до текущего окна.
                while buffer.len() < window {
                    match consumer.pop() {
                        Ok(sample) => buffer.push(sample),
                        Err(_) => {
                            if stop.load(Ordering::Acquire) || consumer.is_abandoned() {
                                return; // захват остановлен
                            }
                            // Данных пока нет: не жжём CPU.
                            std::thread::sleep(Duration::from_millis(1));
                        }
                    }
                }
                let state = {
                    let Ok(mut eng) = engine.lock() else {
                        return; // движок отравлен — выходим тихо
                    };
                    scratch.clear();
                    scratch.extend_from_slice(&buffer[..window]);
                    let state = eng.process_frame(&mut scratch, fs);
                    window = eng.window();
                    state
                    // Мьютекс движка снят до on_state.
                };
                on_state(state);
                // Шаг hop: для тюнера важно «сейчас», история не копится.
                let hop = HOP.min(buffer.len());
                buffer.copy_within(hop.., 0);
                buffer.truncate(buffer.len() - hop);
            }
        })
        .expect("не удалось создать поток анализа")
}

#[cfg(not(target_os = "android"))]
mod cpal_source;
#[cfg(target_os = "android")]
mod oboe;
pub mod tcc;
pub mod watchdog;

pub use watchdog::{StateSink, SupervisedCapture, WatchdogConfig};

/// Бэкенд по умолчанию для desktop-платформ (7.0); Android — oboe (этап 5, 7.1.2).
#[cfg(not(target_os = "android"))]
pub use cpal_source::{list_input_devices, CpalSource, CpalSource as PlatformSource};
#[cfg(target_os = "android")]
pub use oboe::OboeSource as PlatformSource;
#[cfg(target_os = "android")]
pub fn list_input_devices() -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Формат ошибки — контракт с UI: стабильный тегированный объект (0.2).
    #[test]
    fn audio_error_serializes_tagged() {
        assert_eq!(
            serde_json::to_value(AudioError::PermissionDenied).unwrap(),
            serde_json::json!({"kind": "permissionDenied"})
        );
        assert_eq!(
            serde_json::to_value(AudioError::NoInputDevice).unwrap(),
            serde_json::json!({"kind": "noInputDevice"})
        );
        assert_eq!(
            serde_json::to_value(AudioError::Device("busy".into())).unwrap(),
            serde_json::json!({"kind": "device", "message": "busy"})
        );
        assert_eq!(
            serde_json::to_value(AudioError::Stream("timeout".into())).unwrap(),
            serde_json::json!({"kind": "stream", "message": "timeout"})
        );
    }

    use tuner_core::engine::{EngineConfig, Sensitivity};
    use tuner_core::music::{InstrumentRegistry, Tuning};

    fn guitar() -> Tuning {
        InstrumentRegistry::new()
            .tuning("guitar", "standard_e")
            .unwrap()
            .clone()
    }

    fn engine() -> Arc<Mutex<TunerEngine>> {
        Arc::new(Mutex::new(TunerEngine::new(
            EngineConfig {
                tuning: Some(guitar()),
                a4: 440.0,
                tolerance_cents: 3.0,
                gate_db: -45.0,
                sensitivity: Sensitivity::Normal,
                manual_string: None,
                low_range: false,
                hold_ms: 1800,
            },
            48_000,
        )))
    }

    /// Полный конвейер без железа: producer → ring buffer → анализатор → состояние.
    /// Проверяет окно 4096, шаг hop и чистый выход при закрытии producer.
    #[test]
    fn analyzer_emits_states_and_exits_on_close() {
        let (mut producer, consumer) = rtrb::RingBuffer::new(4096 + 4 * HOP);
        let (tx, rx) = std::sync::mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let handle = spawn_analyzer(consumer, engine(), 48_000, stop, move |st| {
            let _ = tx.send(st);
        });

        // Синус E2 (82,41 Гц): окно 4096 + три шага hop.
        let f: f64 = 440.0 * 2f64.powf((40.0 - 69.0) / 12.0);
        for i in 0..4096 + 3 * HOP {
            let t = i as f64 / 48_000.0;
            let _ = producer.push((0.1 * (2.0 * std::f64::consts::PI * f * t).sin()) as f32);
        }

        // Первое окно даёт tracking с нотой E2; дальше — каждые hop.
        let st = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("состояние пришло");
        assert_eq!(st.note_midi, Some(40));
        assert_eq!(st.string_indices, vec![0]);
        assert!(st.cents.unwrap().abs() < 1.5);

        // Закрытие producer останавливает поток без паники.
        drop(producer);
        handle.join().expect("анализатор вышел чисто");
    }

    #[test]
    fn analyzer_survives_window_change_mid_stream() {
        // Балалайка — окно тоже 4096; бас-5 — 16384: смена строя на ходу.
        let (mut producer, consumer) = rtrb::RingBuffer::new(16384 + 4 * HOP);
        let bass = InstrumentRegistry::new()
            .tuning("bass", "bass5")
            .unwrap()
            .clone();
        let engine = engine();
        let (tx, rx) = std::sync::mpsc::channel::<TunerState>();
        let stop = Arc::new(AtomicBool::new(false));
        let handle = spawn_analyzer(consumer, Arc::clone(&engine), 48_000, stop, move |st| {
            let _ = tx.send(st);
        });

        // Начало: гитара, окно 4096 — наполняем и получаем состояние.
        let f: f64 = 440.0 * 2f64.powf((40.0 - 69.0) / 12.0);
        for i in 0..4096 {
            let t = i as f64 / 48_000.0;
            let _ = producer.push((0.1 * (2.0 * std::f64::consts::PI * f * t).sin()) as f32);
        }
        let st = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("кадр гитары");
        assert_eq!(st.note_midi, Some(40));

        // Смена строя на бас: окно вырастает до 16384, анализатор продолжает.
        // Переходные кадры (остаток буфера гитары) пропускаем — ждём чистый B0.
        engine.lock().unwrap().set_config(EngineConfig {
            tuning: Some(bass),
            a4: 440.0,
            tolerance_cents: 3.0,
            gate_db: -45.0,
            sensitivity: Sensitivity::Normal,
            manual_string: None,
            low_range: false,
            hold_ms: 1800,
        });
        let b0: f64 = 440.0 * 2f64.powf((23.0 - 69.0) / 12.0); // B0 = 30,87 Гц
        for i in 0..16384 + 4 * HOP {
            let t = i as f64 / 48_000.0;
            let _ = producer.push((0.2 * (2.0 * std::f64::consts::PI * b0 * t).sin()) as f32);
        }
        let start = std::time::Instant::now();
        let mut saw_b0 = false;
        while !saw_b0 {
            assert!(
                start.elapsed() < Duration::from_secs(20),
                "анализатор не дошёл до чистого кадра баса"
            );
            let st = rx
                .recv_timeout(Duration::from_secs(1))
                .expect("поток состояний жив");
            if st.note_midi == Some(23) {
                saw_b0 = true;
                assert_eq!(st.string_indices, vec![0]);
            }
        }

        drop(producer);
        handle.join().expect("анализатор вышел чисто");
    }

    #[test]
    fn analyzer_exits_on_stop_token_even_if_producer_alive() {
        let (_producer, consumer) = rtrb::RingBuffer::new(4096);
        let stop = Arc::new(AtomicBool::new(false));
        let handle = spawn_analyzer(consumer, engine(), 48_000, Arc::clone(&stop), |_| {});
        stop.store(true, Ordering::Release);
        handle.join().expect("анализатор вышел по стоп-токену");
    }
}
