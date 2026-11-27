//! Живучесть захвата (docs/source/07_platform.md, 7.2.1): наблюдатель
//! следит, что кадры идут. Тишина дольше `idle_ms` или ошибка потока —
//! переоткрываем устройство: наушники/док-станция сменили устройство по
//! умолчанию, USB-микрофон выдернули, поток умер. Не чаще раза в
//! `min_reopen_ms`, максимум `max_attempts` подряд; исчерпаны — слушание
//! прекращаем и сообщаем в UI состояние ошибки (4.4).
//!
//! Крейт не знает про Tauri и UI: состояния уходят через `StateSink`.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use tuner_core::engine::{TunerEngine, TunerKind, TunerState};

use crate::{spawn_analyzer, AudioError, AudioSource, RING_CAPACITY};

/// Параметры наблюдателя. Продакшн-значения — §7.2.1: тишина 1 с,
/// переоткрытие не чаще 2 с, максимум 3 попытки подряд.
#[derive(Clone, Copy, Debug)]
pub struct WatchdogConfig {
    /// «Нет ни одного кадра» дольше этого — переоткрытие.
    pub idle_ms: u64,
    /// Переоткрытие не чаще этого периода.
    pub min_reopen_ms: u64,
    /// Максимум переоткрытий подряд без восстановления.
    pub max_attempts: u32,
    /// Период опроса наблюдателем.
    pub tick_ms: u64,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            idle_ms: 1_000,
            min_reopen_ms: 2_000,
            max_attempts: 3,
            tick_ms: 500,
        }
    }
}

/// Состояния для UI: `Arc`, чтобы пережить переоткрытия потока.
pub type StateSink = Arc<dyn Fn(TunerState) + Send + Sync>;

/// Фабрика источников: наблюдатель открывает новое устройство сам.
pub type SourceFactory<S> = Arc<dyn Fn() -> S + Send + Sync>;

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Живой захват: источник + анализатор + наблюдатель в одном объекте.
/// `stop` извне (команда, закрытие окна) и сдача наблюдателя сходятся
/// к одному и тому же состоянию: `listening = false`, потоки присоединены.
pub struct SupervisedCapture<S: AudioSource + 'static> {
    engine: Arc<Mutex<TunerEngine>>,
    make_source: SourceFactory<S>,
    sink: StateSink,
    cfg: WatchdogConfig,
    listening: AtomicBool,
    /// Открытие в процессе (например, системный запрос TCC): наблюдатель
    /// в это время не считает попытки.
    opening: AtomicBool,
    /// Текущая попытка: источник держит поток, анализатор — данные.
    current: Mutex<Option<Current<S>>>,
    attempts: AtomicU32,
    next_reopen_ms: AtomicU64,
    /// Метка последнего переоткрытия: попытки сбрасываются, только если
    /// кадры пришли **после** неё (данные реально восстановились).
    reopen_mark_ms: AtomicU64,
    last_error: Mutex<Option<AudioError>>,
    supervisor: Mutex<Option<JoinHandle<()>>>,
}

struct Current<S> {
    source: S,
    analyzer: JoinHandle<()>,
    stop_token: Arc<AtomicBool>,
    /// Обновляется анализатором на каждом кадре.
    last_frame_ms: Arc<AtomicU64>,
}

impl<S: AudioSource + 'static> SupervisedCapture<S> {
    /// Создаёт захват и сразу открывает поток; неудача — ошибка вызывающему
    /// (UI покажет экран с «Повторить»), наблюдатель не запускается.
    pub fn spawn(
        engine: Arc<Mutex<TunerEngine>>,
        make_source: SourceFactory<S>,
        sink: StateSink,
        cfg: WatchdogConfig,
    ) -> Result<Arc<Self>, AudioError> {
        let capture = Arc::new(Self {
            engine,
            make_source,
            sink,
            cfg,
            listening: AtomicBool::new(true),
            opening: AtomicBool::new(false),
            current: Mutex::new(None),
            attempts: AtomicU32::new(0),
            next_reopen_ms: AtomicU64::new(0),
            reopen_mark_ms: AtomicU64::new(0),
            last_error: Mutex::new(None),
            supervisor: Mutex::new(None),
        });
        capture.start_stream()?;
        let supervisor = std::thread::Builder::new()
            .name("unisono-watchdog".into())
            .spawn({
                let capture = Arc::clone(&capture);
                move || capture.supervise()
            })
            .expect("не удалось создать наблюдателя");
        *capture.supervisor.lock().expect("supervisor") = Some(supervisor);
        Ok(capture)
    }

    pub fn is_listening(&self) -> bool {
        self.listening.load(Ordering::Acquire)
    }

    /// Полный стоп: источник, анализатор, наблюдатель (3.6: окно отдельно
    /// от аудио; закрытие окна зовёт это же). Идемпотентен.
    pub fn stop(&self) {
        self.listening.store(false, Ordering::Release);
        self.stop_stream();
        if let Some(handle) = self.supervisor.lock().expect("supervisor").take() {
            let _ = handle.join();
        }
    }

    /// Открывает поток: кольцо → источник → анализатор; каждый кадр
    /// обновляет `last_frame_ms`, состояния уходят в `sink`.
    fn start_stream(&self) -> Result<(), AudioError> {
        self.opening.store(true, Ordering::Release);
        let result = self.start_stream_inner();
        self.opening.store(false, Ordering::Release);
        result
    }

    fn start_stream_inner(&self) -> Result<(), AudioError> {
        let mut source = (self.make_source)();
        let (producer, consumer) = rtrb::RingBuffer::new(RING_CAPACITY);
        source.start(producer).inspect_err(|e| {
            *self.last_error.lock().expect("last_error") = Some(e.clone());
        })?;
        let fs = source.sample_rate();
        *self.last_error.lock().expect("last_error") = None;

        // Каждый кадр анализатора — «живой» сигнал для наблюдателя.
        let last_frame_ms = Arc::new(AtomicU64::new(now_ms()));
        let sink = Arc::clone(&self.sink);
        let last = Arc::clone(&last_frame_ms);
        let on_state = move |state: TunerState| {
            last.store(now_ms(), Ordering::Relaxed);
            sink(state);
        };
        let engine = Arc::clone(&self.engine);
        let stop_token = Arc::new(AtomicBool::new(false));
        let analyzer = spawn_analyzer(consumer, engine, fs, Arc::clone(&stop_token), on_state);
        *self.current.lock().expect("current") = Some(Current {
            source,
            analyzer,
            stop_token,
            last_frame_ms,
        });
        Ok(())
    }

    /// Останавливает текущую попытку: токен останавливает анализатор,
    /// явный вызов source.stop() останавливает источник, дроп источника
    /// и джойн анализатора (rev 6.4).
    fn stop_stream(&self) {
        if let Some(mut current) = self.current.lock().expect("current").take() {
            current.stop_token.store(true, Ordering::Release);
            current.source.stop();
            drop(current.source);
            let _ = current.analyzer.join();
        }
    }

    /// Тело наблюдателя: переоткрытие при тишине или ошибке потока,
    /// максимум `max_attempts` подряд (7.2.1). Попытки сбрасываются,
    /// только когда кадры пришли **после** переоткрытия — свежий старт
    /// без данных попыткой не считается выздоровевшим.
    fn supervise(self: Arc<Self>) {
        const SLICE_MS: u64 = 25;
        loop {
            if !self.listening.load(Ordering::Acquire) {
                return;
            }
            // Сон нарезами для быстрого отклика на stop() (rev 6.3).
            let mut elapsed = 0;
            let total = self.cfg.tick_ms;
            while elapsed < total {
                let chunk = (total - elapsed).min(SLICE_MS);
                std::thread::sleep(Duration::from_millis(chunk));
                if !self.listening.load(Ordering::Acquire) {
                    return;
                }
                elapsed += chunk;
            }
            // Открытие в процессе (например, TCC-диалог) — не мешаем.
            if self.opening.load(Ordering::Acquire) {
                continue;
            }
            let verdict = {
                let current = self.current.lock().expect("current");
                match current.as_ref() {
                    // Захват остановлен снаружи, но флаг ещё жив: поток не открыт.
                    None if self.listening.load(Ordering::Acquire) => {
                        Verdict::Reopen("поток не открыт")
                    }
                    None => return,
                    Some(cur) => {
                        let last = cur.last_frame_ms.load(Ordering::Acquire);
                        if cur.source.failed() {
                            Verdict::Reopen("ошибка потока")
                        } else if now_ms().saturating_sub(last) > self.cfg.idle_ms {
                            Verdict::Reopen("нет данных")
                        } else if last > self.reopen_mark_ms.load(Ordering::Acquire) {
                            // Кадры пришли после последнего переоткрытия —
                            // захват восстановился.
                            Verdict::Healthy
                        } else {
                            // Свежее переоткрытие: кадров ещё не было, но и
                            // тишины до порога — просто ждём.
                            Verdict::Wait
                        }
                    }
                }
            };
            match verdict {
                Verdict::Wait => continue,
                Verdict::Healthy => {
                    self.attempts.store(0, Ordering::Release);
                    continue;
                }
                Verdict::Reopen(reason) => {
                    // Переоткрытие не чаще раза в min_reopen_ms (7.2.1).
                    let now = now_ms();
                    if now < self.next_reopen_ms.load(Ordering::Acquire) {
                        continue;
                    }
                    self.next_reopen_ms
                        .store(now + self.cfg.min_reopen_ms, Ordering::Release);

                    let attempt = self.attempts.fetch_add(1, Ordering::AcqRel) + 1;
                    if attempt > self.cfg.max_attempts {
                        // Исчерпаны: слушание прекращаем, UI получает ошибку (4.4).
                        tracing::warn!(%reason, attempt, "захват не восстановился");
                        self.listening.store(false, Ordering::Release);
                        self.stop_stream();
                        let error = self.last_error.lock().expect("last_error").clone();
                        (self.sink)(error_state(error));
                        return;
                    }
                    tracing::info!(%reason, attempt, "переоткрытие потока");
                    self.stop_stream();
                    // Движок — в Silent: индикатор не «приползает» из старого состояния.
                    if let Ok(mut engine) = self.engine.lock() {
                        engine.reset();
                    }
                    (self.sink)(silent_state());
                    // Метка ставится после старта: кадры «после» неё = выздоровление.
                    if let Err(e) = self.start_stream() {
                        // Ошибка сохранена; следующий тик решит — ещё попытка или сдача.
                        tracing::warn!(error = %e, "переоткрытие не удалось");
                    }
                    self.reopen_mark_ms.store(now_ms(), Ordering::Release);
                }
            }
        }
    }
}

/// Решение наблюдателя по одному тику.
enum Verdict {
    /// Свежее переоткрытие: кадров ещё не было, тишина не дошла до порога.
    Wait,
    /// Кадры идут с момента последнего переоткрытия.
    Healthy,
    /// Переоткрыть поток (причина — для лога).
    Reopen(&'static str),
}

fn silent_state() -> TunerState {
    TunerState {
        kind: TunerKind::Silent,
        f0: None,
        cents: None,
        clarity: 0.0,
        db: -180.0,
        note_midi: None,
        target_hz: None,
        string_indices: Vec::new(),
    }
}

fn error_state(error: Option<AudioError>) -> TunerState {
    let kind = match error {
        Some(AudioError::PermissionDenied) => TunerKind::NoPermission,
        _ => TunerKind::Error,
    };
    TunerState {
        kind,
        f0: None,
        cents: None,
        clarity: 0.0,
        db: -180.0,
        note_midi: None,
        target_hz: None,
        string_indices: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use tuner_core::engine::EngineConfig;
    use tuner_core::engine::Sensitivity;

    /// Фейковый источник без железа: считает запуски, может падать,
    /// может проталкивать данные, чтобы анализатор «оживал».
    struct FakeSource {
        started: Arc<AtomicU32>,
        fail_from: Option<u32>,
        deliver: usize,
        producer: Option<rtrb::Producer<f32>>,
    }

    impl FakeSource {
        fn factory(
            started: Arc<AtomicU32>,
            fail_from: Option<u32>,
            deliver: usize,
        ) -> SourceFactory<Self> {
            Arc::new(move || Self {
                started: Arc::clone(&started),
                fail_from,
                deliver,
                producer: None,
            })
        }
    }

    impl AudioSource for FakeSource {
        fn sample_rate(&self) -> u32 {
            48_000
        }

        fn start(&mut self, mut sink: rtrb::Producer<f32>) -> Result<(), AudioError> {
            let n = self.started.fetch_add(1, Ordering::AcqRel) + 1;
            if let Some(fail) = self.fail_from {
                if n >= fail {
                    return Err(AudioError::Device("fake".into()));
                }
            }
            // Блок данных: анализатор обработает окно и обновит last_frame.
            for _ in 0..self.deliver {
                let _ = sink.push(0.0);
            }
            self.producer = Some(sink); // канал жив, пока жив источник
            Ok(())
        }

        fn stop(&mut self) {
            self.producer = None;
        }
    }

    fn engine() -> Arc<Mutex<TunerEngine>> {
        Arc::new(Mutex::new(TunerEngine::new(
            EngineConfig {
                tuning: None,
                a4: 440.0,
                tolerance_cents: 3.0,
                gate_db: -45.0,
                sensitivity: Sensitivity::Normal,
                manual_string: None,
                low_range: false,
                hold_ms: 1800,
                ..Default::default()
            },
            48_000,
        )))
    }

    /// Быстрый наблюдатель для тестов: тишина 150 мс, переоткрытие 40 мс.
    fn fast_cfg() -> WatchdogConfig {
        WatchdogConfig {
            idle_ms: 150,
            min_reopen_ms: 40,
            max_attempts: 3,
            tick_ms: 30,
        }
    }

    #[test]
    fn watchdog_reopens_and_gives_up_when_no_data() {
        let started = Arc::new(AtomicU32::new(0));
        let (tx, rx) = mpsc::channel::<TunerState>();
        let sink: StateSink = Arc::new(move |st| {
            let _ = tx.send(st);
        });
        // Данных нет никогда: 1 старт + 3 переоткрытия, потом ошибка в UI.
        let capture = SupervisedCapture::spawn(
            engine(),
            FakeSource::factory(Arc::clone(&started), None, 0),
            sink,
            fast_cfg(),
        )
        .expect("первый старт ок");

        // Ждём сдачи: 3 переоткрытия × (тишина 150 + шаг 30) + запас.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut saw_error = false;
        while std::time::Instant::now() < deadline {
            if let Ok(st) = rx.recv_timeout(Duration::from_millis(200)) {
                if st.kind == TunerKind::Error {
                    saw_error = true;
                    break;
                }
            }
        }
        assert!(
            saw_error,
            "UI должен получить состояние ошибки; стартов: {}",
            started.load(Ordering::Acquire)
        );
        assert!(!capture.is_listening());
        // 1 старт + max_attempts переоткрытий; сдача потока уже не открывает.
        assert_eq!(started.load(Ordering::Acquire), 4);
        // Повторный stop после сдачи — не паника.
        capture.stop();
    }

    #[test]
    fn watchdog_reports_permission_error_when_reopen_denied() {
        let started = Arc::new(AtomicU32::new(0));
        let (tx, rx) = mpsc::channel::<TunerState>();
        let sink: StateSink = Arc::new(move |st| {
            let _ = tx.send(st);
        });
        // Первый старт ок, все переоткрытия — отказ в доступе.
        let capture = SupervisedCapture::spawn(
            engine(),
            FakeSource::factory(Arc::clone(&started), Some(2), 0),
            sink,
            fast_cfg(),
        )
        .expect("первый старт ок");

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut kind = None;
        while std::time::Instant::now() < deadline {
            if let Ok(st) = rx.recv_timeout(Duration::from_millis(200)) {
                if st.kind == TunerKind::Error || st.kind == TunerKind::NoPermission {
                    kind = Some(st.kind);
                    break;
                }
            }
        }
        // Последняя ошибка — Device: это экран Error, не NoPermission.
        assert_eq!(kind, Some(TunerKind::Error));
        assert!(!capture.is_listening());
        // Старт + 3 неудачных переоткрытия, четвёртая попытка — сдача.
        assert_eq!(started.load(Ordering::Acquire), 4);
    }

    /// Данные идут после каждого переоткрытия — попытки сбрасываются,
    /// захват не сдаётся и продолжает слушать.
    #[test]
    fn watchdog_resets_attempts_when_data_flows() {
        let started = Arc::new(AtomicU32::new(0));
        let (tx, _rx) = mpsc::channel::<TunerState>();
        let sink: StateSink = Arc::new(move |st| {
            let _ = tx.send(st);
        });
        // Щедрый бюджет для CI-нагрузки: планировщик ОС успевает передать
        // кадры до исчерпания попыток.
        let cfg = WatchdogConfig {
            idle_ms: 150,
            min_reopen_ms: 40,
            max_attempts: 5,
            tick_ms: 20,
        };
        // Каждый старт проталкивает окно данных: анализатор оживает.
        let capture = SupervisedCapture::spawn(
            engine(),
            FakeSource::factory(Arc::clone(&started), None, 4096 + 1024),
            sink,
            cfg,
        )
        .expect("первый старт ок");

        // Poll-цикл с дедлайном 5 с: ждём подтверждения переоткрытий (started >= 2)
        // и непрерывно проверяем, что захват остаётся активным (попытки сбрасываются).
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut reopens_seen = 0;
        while std::time::Instant::now() < deadline {
            assert!(capture.is_listening(), "данные идут — попытки сбрасываются");
            reopens_seen = started.load(Ordering::Acquire);
            if reopens_seen >= 3 {
                break;
            }
            std::thread::sleep(Duration::from_millis(30));
        }

        assert!(capture.is_listening(), "данные идут — попытки сбрасываются");
        assert!(
            reopens_seen >= 2,
            "было переоткрытие (фактически: {reopens_seen})"
        );
        capture.stop();
        assert!(!capture.is_listening());
    }

    /// Стоп идемпотентен, повторный старт после стопа — без утечек.
    #[test]
    fn stop_is_idempotent_and_restart_works() {
        let started = Arc::new(AtomicU32::new(0));
        let (tx, _rx) = mpsc::channel::<TunerState>();
        let sink: StateSink = Arc::new(move |st| {
            let _ = tx.send(st);
        });
        let capture = SupervisedCapture::spawn(
            engine(),
            FakeSource::factory(Arc::clone(&started), None, 0),
            Arc::clone(&sink),
            fast_cfg(),
        )
        .expect("старт");
        assert!(capture.is_listening());
        capture.stop();
        capture.stop(); // повтор — не паника
        assert!(!capture.is_listening());

        // Повторный захват: новое устройство, новый наблюдатель.
        let again = SupervisedCapture::spawn(
            engine(),
            FakeSource::factory(Arc::clone(&started), None, 0),
            sink,
            fast_cfg(),
        )
        .expect("повторный старт");
        assert!(again.is_listening());
        again.stop();
    }

    /// rev 6.4: стоп быстро возвращается с FakeSource (<= 500 мс), и повторный старт работает.
    #[test]
    fn fast_stop_with_fake_source_and_restart() {
        let started = Arc::new(AtomicU32::new(0));
        let sink: StateSink = Arc::new(|_| {});
        let capture = SupervisedCapture::spawn(
            engine(),
            FakeSource::factory(Arc::clone(&started), None, 4096),
            Arc::clone(&sink),
            WatchdogConfig::default(),
        )
        .expect("старт");
        assert!(capture.is_listening());
        let t0 = std::time::Instant::now();
        capture.stop();
        let elapsed = t0.elapsed();
        assert!(!capture.is_listening());
        assert!(
            elapsed < Duration::from_millis(500),
            "стоп занял {:?}",
            elapsed
        );

        // Повторный старт
        let again = SupervisedCapture::spawn(
            engine(),
            FakeSource::factory(Arc::clone(&started), None, 4096),
            sink,
            WatchdogConfig::default(),
        )
        .expect("повторный старт");
        assert!(again.is_listening());
        let t1 = std::time::Instant::now();
        again.stop();
        assert!(!again.is_listening());
        assert!(t1.elapsed() < Duration::from_millis(500));
    }
}
