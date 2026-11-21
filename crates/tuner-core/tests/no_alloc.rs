//! Тест отсутствия аллокаций в горячем пути (R-6, 4.3.7).
//!
//! Счётчик аллокаций — обёртка над System; `unsafe` живёт только в этом
//! тестовом harness-файле, в библиотеке `#![forbid(unsafe_code)]`.
//! После прогрева `process_frame` и `SmoothingFilter::push` не должны
//! выделять память. Тест в бинарнике один: параллельных аллокаций нет.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use tuner_core::dsp::{DetectorConfig, Estimate, PitchDetector, SmoothingFilter};

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const FS: u32 = 48_000;

fn sine(freq: f32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| 0.1 * (2.0 * std::f32::consts::PI * freq * i as f32 / FS as f32).sin())
        .collect()
}

#[test]
fn hot_path_does_not_allocate() {
    let mut d4 = PitchDetector::new(DetectorConfig::new(4096, FS, 82.41));
    let mut d16 = PitchDetector::new(DetectorConfig::new(16384, FS, 30.87));
    let mut smooth = SmoothingFilter::new();
    let src4 = sine(440.0, 4096);
    let src16 = sine(30.87, 16384);
    let mut buf4 = vec![0f32; 4096];
    let mut buf16 = vec![0f32; 16384];

    // Прогрев: состояния фильтров и ветви кода уже exercised.
    for _ in 0..20 {
        buf4.copy_from_slice(&src4);
        buf16.copy_from_slice(&src16);
        d4.process_frame(&mut buf4, FS);
        d16.process_frame(&mut buf16, FS);
        smooth.push(Some(440.0), 21.3);
    }

    let before = ALLOCS.load(Ordering::Relaxed);
    for _ in 0..100 {
        buf4.copy_from_slice(&src4);
        buf16.copy_from_slice(&src16);
        let e4 = d4.process_frame(&mut buf4, FS);
        let e16 = d16.process_frame(&mut buf16, FS);
        let f = match e4 {
            Some(Estimate::Pitch { f0, .. }) => Some(f64::from(f0)),
            _ => None,
        };
        let s = smooth.push(f, 21.3);
        std::hint::black_box((e4, e16, s));
    }
    let after = ALLOCS.load(Ordering::Relaxed);
    assert_eq!(
        after,
        before,
        "горячий путь аллоцировал {} раз",
        after - before
    );
}
