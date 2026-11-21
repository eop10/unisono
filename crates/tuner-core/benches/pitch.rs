//! Criterion-бенчмарки детектора (docs/source/03_architecture.md, 3.7).
//!
//! `cargo bench -p tuner-core` — бюджеты 3 / 6 / 14 мс на кадр.

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use tuner_core::dsp::{DetectorConfig, PitchDetector};

const FS: u32 = 48_000;

fn harmonic(freq: f32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let t = i as f32 / FS as f32;
            let mut x = 0.0;
            for k in 1..=6u32 {
                x += (2.0 * std::f32::consts::PI * freq * k as f32 * t).sin() / k as f32;
            }
            x * 0.1
        })
        .collect()
}

fn bench_pitch(c: &mut Criterion) {
    let mut group = c.benchmark_group("pitch");
    for (window, f_min) in [(4096usize, 82.41f32), (8192, 65.41), (16384, 30.87)] {
        let src = harmonic(f_min, window);
        group.bench_function(format!("window_{window}"), |b| {
            let mut det = PitchDetector::new(DetectorConfig::new(window, FS, f_min));
            b.iter_batched(
                || src.clone(),
                |mut frame| det.process_frame(&mut frame, FS),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, bench_pitch);
criterion_main!(benches);
