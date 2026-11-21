//! Бюджеты производительности (docs/source/03_architecture.md, 3.7).
//!
//! Запуск локально: `cargo test -p tuner-core --release --test perf -- --ignored`.
//! В CI не выполняется (зависит от машины); непрерывный замер — criterion-бенч.

use std::time::Instant;
use tuner_core::dsp::{DetectorConfig, PitchDetector};

const FS: u32 = 48_000;

fn sine(freq: f32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| 0.1 * (2.0 * std::f32::consts::PI * freq * i as f32 / FS as f32).sin())
        .collect()
}

#[test]
#[ignore = "зависит от машины; запускать локально в release"]
fn perf_budget_per_window() {
    for (window, f_min, budget_ms) in [
        (4096usize, 82.41f32, 3.0f64),
        (8192, 65.41, 6.0),
        (16384, 30.87, 14.0),
    ] {
        let src = sine(f_min, window);
        let mut buf = vec![0f32; window];
        let mut det = PitchDetector::new(DetectorConfig::new(window, FS, f_min));
        for _ in 0..10 {
            buf.copy_from_slice(&src);
            det.process_frame(&mut buf, FS);
        }
        let frames = 100;
        let t0 = Instant::now();
        for _ in 0..frames {
            buf.copy_from_slice(&src);
            std::hint::black_box(det.process_frame(&mut buf, FS));
        }
        let avg_ms = t0.elapsed().as_secs_f64() * 1000.0 / frames as f64;
        println!("window {window}: {avg_ms:.3} ms/frame (budget {budget_ms} ms)");
        assert!(
            avg_ms <= budget_ms,
            "window {window}: {avg_ms:.3} ms > {budget_ms} ms"
        );
    }
}
