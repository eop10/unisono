//! NSDF — нормированная автокорреляционная функция (docs/source/04_dsp.md, 4.3.2).
//!
//! `n(τ) = 2·Σ x_j·x_{j+τ} / Σ (x_j² + x_{j+τ}²)`, значения в [−1, 1].
//! Знаменатель считается через префиксные суммы энергий за O(N), поэтому
//! горячий цикл — только числитель (4.5: один проход, без промежуточных Vec).

/// Префиксные суммы квадратов: `out[k] = Σ_{j<k} x_j²`, длина `frame.len() + 1`.
pub fn energy_prefix(frame: &[f32], out: &mut [f64]) {
    out[0] = 0.0;
    for (j, &x) in frame.iter().enumerate() {
        out[j + 1] = out[j] + f64::from(x) * f64::from(x);
    }
}

/// NSDF для `τ ∈ [tau_min, tau_max]`; `out` индексируется значением τ.
///
/// Буферы передаются снаружи: функция не аллоцирует.
pub fn nsdf(frame: &[f32], prefix: &[f64], tau_min: usize, tau_max: usize, out: &mut [f32]) {
    let n = frame.len();
    let total = prefix[n];
    for tau in tau_min..=tau_max {
        let m = n - tau;
        // Четыре независимых аккумулятора: разрывает цепочку зависимостей
        // по сложению (латентность FMA) и даёт LLVM векторизовать цикл;
        // заодно точнее последовательной суммы (4.5).
        let mut s0 = 0f32;
        let mut s1 = 0f32;
        let mut s2 = 0f32;
        let mut s3 = 0f32;
        let mut j = 0;
        while j + 4 <= m {
            s0 += frame[j] * frame[j + tau];
            s1 += frame[j + 1] * frame[j + 1 + tau];
            s2 += frame[j + 2] * frame[j + 2 + tau];
            s3 += frame[j + 3] * frame[j + 3 + tau];
            j += 4;
        }
        while j < m {
            s0 += frame[j] * frame[j + tau];
            j += 1;
        }
        let num = (s0 + s1) + (s2 + s3);
        // Σ_{j<m} x_j² + Σ_{j<m} x_{j+τ}² = prefix[m] + (prefix[n] − prefix[τ]).
        let den = prefix[m] + total - prefix[tau];
        out[tau] = if den > 0.0 {
            (2.0 * f64::from(num) / den) as f32
        } else {
            0.0
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, n: usize, fs: u32) -> Vec<f32> {
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / fs as f32).sin())
            .collect()
    }

    #[test]
    fn nsdf_peaks_at_true_period() {
        let fs = 48_000u32;
        let frame = sine(440.0, 4096, fs);
        let mut prefix = vec![0f64; frame.len() + 1];
        energy_prefix(&frame, &mut prefix);
        let mut out = vec![0f32; 2048];
        nsdf(&frame, &prefix, 34, 2047, &mut out);
        let tau_true = fs as f32 / 440.0; // ≈ 109.09
        let k = tau_true.round() as usize;
        assert!(out[k] > 0.99, "n(τ*) = {}", out[k]);
        // Соседние τ заметно ниже: пик острый.
        assert!(out[k] > out[k - 5]);
        assert!(out[k] > out[k + 5]);
    }

    #[test]
    fn nsdf_bounded_and_zero_on_silence() {
        let frame = vec![0f32; 4096];
        let mut prefix = vec![0f64; frame.len() + 1];
        energy_prefix(&frame, &mut prefix);
        let mut out = vec![0f32; 2048];
        nsdf(&frame, &prefix, 34, 2047, &mut out);
        assert!(out[34..=2047].iter().all(|&v| v == 0.0));
    }

    #[test]
    fn nsdf_period_double_for_octave_signal() {
        // Синус + сильная вторая гармоника: пики на τ и 2τ.
        let fs = 48_000u32;
        let frame: Vec<f32> = (0..4096)
            .map(|i| {
                let t = i as f32 / fs as f32;
                (2.0 * std::f32::consts::PI * 220.0 * t).sin()
                    + 0.8 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()
            })
            .collect();
        let mut prefix = vec![0f64; frame.len() + 1];
        energy_prefix(&frame, &mut prefix);
        let mut out = vec![0f32; 2048];
        nsdf(&frame, &prefix, 34, 2047, &mut out);
        let k = (fs as f32 / 220.0).round() as usize;
        assert!(out[k] > 0.9);
        assert!(out[2 * k] > 0.9);
    }
}
