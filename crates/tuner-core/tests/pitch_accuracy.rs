//! Точность DSP на WAV-фикстурах (docs/source/04_dsp.md, 4.6).
//!
//! Метрики: maxAbsCents по устойчивой части (после 100 мс), octaveErrorRate,
//! detectionRate. Допуски: 30–65 Гц ±3, 65–80 ±2, 80–700 ±1, 700–1400 ±2 цента;
//! harmonic/KS ±1, overdrive ±2, noise SNR 20/10/6 ±1/2/4. Речь и тишина —
//! ноль ложных нот. Окно и f_min берутся из манифеста (генератор 4.2.3).

use hound::WavReader;
use tuner_core::dsp::{DetectorConfig, Estimate, PitchDetector, SmoothingFilter};

const FS: u32 = 48_000;
/// Шаг анализа как в runtime (4.2.1): 1024 отсчёта, ≈ 47 кадров/с.
const HOP: usize = 1024;

struct Row {
    file: String,
    kind: String,
    f0: f64,
    snr: Option<u32>,
    window: usize,
    dur: f64,
}

fn manifest() -> Vec<Row> {
    include_str!("fixtures/manifest.csv")
        .lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let c: Vec<&str> = l.split(',').collect();
            Row {
                file: c[0].to_string(),
                kind: c[1].to_string(),
                f0: c[2].parse().unwrap(),
                snr: c[3].parse().ok(),
                window: c[4].parse().unwrap(),
                dur: c[5].parse().unwrap(),
            }
        })
        .collect()
}

fn tolerance(kind: &str, f0: f64, snr: Option<u32>) -> f64 {
    match kind {
        "sine" => {
            if f0 < 65.0 {
                3.0
            } else if f0 < 80.0 {
                2.0
            } else if f0 <= 700.0 {
                1.0
            } else {
                2.0
            }
        }
        "harmonic" | "ks" => 1.0,
        "overdrive" => 2.0,
        "noise" => match snr {
            Some(20) => 1.0,
            Some(10) => 2.0,
            _ => 4.0,
        },
        _ => 0.0,
    }
}

fn min_detection(kind: &str, snr: Option<u32>) -> f64 {
    match kind {
        "sine" | "harmonic" | "ks" | "overdrive" => 0.85,
        "noise" => match snr {
            Some(20) => 0.8,
            Some(10) => 0.7,
            _ => 0.5,
        },
        _ => 0.0,
    }
}

#[test]
fn fixtures_accuracy() {
    let mut failures: Vec<String> = Vec::new();
    let mut report: Vec<String> = Vec::new();
    for row in manifest() {
        let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), row.file);
        let mut reader = WavReader::open(&path).unwrap_or_else(|e| panic!("{}: {e}", row.file));
        let samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| f32::from(s.unwrap()) / 32768.0)
            .collect();
        let f_min = if row.f0 > 0.0 { row.f0 as f32 } else { 30.0 };
        let mut det = PitchDetector::new(DetectorConfig::new(row.window, FS, f_min));
        let mut smooth = SmoothingFilter::new();
        let mut buf = vec![0f32; row.window];
        let dt_ms = HOP as f64 * 1000.0 / f64::from(FS);

        let mut steady = 0usize;
        let mut pitches = 0usize;
        let mut octave_errors = 0usize;
        let mut max_cents = 0f64;
        let mut false_notes = 0usize;

        let mut start = 0usize;
        while start + row.window <= samples.len() {
            buf.copy_from_slice(&samples[start..start + row.window]);
            let t = start as f64 / f64::from(FS);
            let est = det.process_frame(&mut buf, FS);
            // Метрики — по выходу конвейера: детектор + сглаживание (4.3.5).
            let smoothed = match est {
                Some(Estimate::Pitch { f0, .. }) => smooth.push(Some(f64::from(f0)), dt_ms),
                _ => smooth.push(None, dt_ms),
            };
            let in_steady = t >= 0.1 && t <= row.dur - 0.02 && !(row.kind == "ks" && t > 0.45);
            if let Some(sf) = smoothed {
                if row.f0 == 0.0 {
                    false_notes += 1; // речь/тишина: нот быть не должно
                } else if in_steady {
                    pitches += 1;
                    let err = 1200.0 * (sf / row.f0).log2();
                    if (err / 1200.0).round().abs() >= 1.0 {
                        octave_errors += 1;
                    }
                    max_cents = max_cents.max(err.abs());
                }
            }
            if in_steady && row.f0 > 0.0 {
                steady += 1;
            }
            start += HOP;
        }

        if row.f0 == 0.0 {
            if false_notes > 0 {
                failures.push(format!(
                    "{}: {false_notes} ложных нот (речь/тишина)",
                    row.file
                ));
            }
            report.push(format!("{}: false_notes={false_notes}", row.file));
            continue;
        }
        let rate = pitches as f64 / steady.max(1) as f64;
        let tol = tolerance(&row.kind, row.f0, row.snr);
        if max_cents > tol {
            failures.push(format!("{}: maxAbsCents {max_cents:.2} > {tol}", row.file));
        }
        if octave_errors > 0 {
            failures.push(format!("{}: {octave_errors} октавных ошибок", row.file));
        }
        if rate < min_detection(&row.kind, row.snr) {
            failures.push(format!("{}: detectionRate {rate:.2} ниже порога", row.file));
        }
        report.push(format!(
            "{}: maxCents={max_cents:.2} oct={octave_errors} det={rate:.2}",
            row.file
        ));
    }
    for line in &report {
        println!("{line}");
    }
    assert!(
        failures.is_empty(),
        "провалы точности:\n{}",
        failures.join("\n")
    );
}
