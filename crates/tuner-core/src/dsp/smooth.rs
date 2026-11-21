//! Сглаживание оценки частоты (docs/source/04_dsp.md, 4.3.5).
//!
//! Медиана 5 кадров + EMA α = 0,30 по `y = 1200·log2(f/440)` — абсолютные центы
//! относительно A4 = 440, без привязки к текущей опорной частоте: смена A4
//! не требует пересчёта истории. Разрыв сигнала дольше 400 мс сбрасывает фильтр.

/// Окно медианы (кадры).
const MEDIAN_LEN: usize = 5;
/// Коэффициент EMA.
const ALPHA: f64 = 0.30;
/// Разрыв сигнала, после которого фильтр сбрасывается (мс).
const RESET_SILENCE_MS: f64 = 400.0;

/// Медиана-5 + EMA в логарифмической шкале. Без аллокаций в `push`.
#[derive(Clone, Debug)]
pub struct SmoothingFilter {
    history: [f64; MEDIAN_LEN],
    len: usize,
    ema: Option<f64>,
    silence_ms: f64,
}

impl Default for SmoothingFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl SmoothingFilter {
    pub fn new() -> Self {
        Self {
            history: [0.0; MEDIAN_LEN],
            len: 0,
            ema: None,
            silence_ms: 0.0,
        }
    }

    /// Кадр с оценкой `f0` (Гц) или `None` (тишина/недостоверно); `dt_ms` — шаг
    /// между кадрами. Возвращает сглаженную частоту или `None`.
    pub fn push(&mut self, f0: Option<f64>, dt_ms: f64) -> Option<f64> {
        let f = match f0 {
            Some(f) if f > 0.0 => f,
            _ => {
                self.silence_ms += dt_ms;
                if self.silence_ms > RESET_SILENCE_MS {
                    self.reset();
                }
                return None;
            }
        };
        self.silence_ms = 0.0;

        let y = 1200.0 * (f / 440.0).log2();
        // Сдвигаем окно медианы (5 элементов, без аллокаций).
        self.history.copy_within(1..MEDIAN_LEN, 0);
        self.history[MEDIAN_LEN - 1] = y;
        self.len = (self.len + 1).min(MEDIAN_LEN);

        let mut sorted = [0f64; MEDIAN_LEN];
        sorted[..self.len].copy_from_slice(&self.history[MEDIAN_LEN - self.len..]);
        // Сортировка вставками: ≤ 5 элементов.
        for i in 1..self.len {
            let v = sorted[i];
            let mut j = i;
            while j > 0 && sorted[j - 1] > v {
                sorted[j] = sorted[j - 1];
                j -= 1;
            }
            sorted[j] = v;
        }
        let median = sorted[self.len / 2];

        let s = match self.ema {
            None => median,
            Some(prev) => ALPHA * median + (1.0 - ALPHA) * prev,
        };
        self.ema = Some(s);
        Some(440.0 * 2f64.powf(s / 1200.0))
    }

    /// Сброс истории и EMA (разрыв сигнала, смена инструмента/строя).
    pub fn reset(&mut self) {
        self.len = 0;
        self.ema = None;
        self.silence_ms = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOP_MS: f64 = 1024.0 / 48.0; // ≈ 21,3 мс при 48 кГц

    #[test]
    fn first_frame_passes_through() {
        let mut s = SmoothingFilter::new();
        let out = s.push(Some(440.0), HOP_MS).unwrap();
        assert!((out - 440.0).abs() < 1e-9);
    }

    #[test]
    fn median_rejects_single_outlier() {
        let mut s = SmoothingFilter::new();
        for _ in 0..2 {
            s.push(Some(440.0), HOP_MS);
        }
        s.push(Some(880.0), HOP_MS); // октавный выброс
        let out = s.push(Some(440.0), HOP_MS).unwrap();
        let cents = 1200.0 * (out / 440.0).log2();
        assert!(cents.abs() < 5.0, "cents = {cents}");
    }

    #[test]
    fn ema_converges_to_new_pitch() {
        let mut s = SmoothingFilter::new();
        for _ in 0..10 {
            s.push(Some(440.0), HOP_MS);
        }
        let mut out = 440.0;
        for _ in 0..30 {
            out = s.push(Some(466.16), HOP_MS).unwrap();
        }
        let cents = 1200.0 * (out / 466.16).log2();
        assert!(cents.abs() < 1.0, "cents = {cents}");
    }

    #[test]
    fn silence_over_400ms_resets() {
        let mut s = SmoothingFilter::new();
        for _ in 0..5 {
            s.push(Some(440.0), HOP_MS);
        }
        // 3 кадра тишины по 200 мс = 600 мс > 400 мс.
        for _ in 0..3 {
            assert_eq!(s.push(None, 200.0), None);
        }
        // После сброса первый кадр новой ноты проходит без «приползания».
        let out = s.push(Some(880.0), HOP_MS).unwrap();
        assert!((out - 880.0).abs() < 1e-9);
    }

    #[test]
    fn short_silence_does_not_reset() {
        let mut s = SmoothingFilter::new();
        for _ in 0..5 {
            s.push(Some(440.0), HOP_MS);
        }
        s.push(None, 200.0);
        s.push(None, 150.0); // 350 мс < 400 мс
        let out = s.push(Some(440.0), HOP_MS).unwrap();
        let cents = 1200.0 * (out / 440.0).log2();
        assert!(cents.abs() < 1e-6, "cents = {cents}");
    }
}
