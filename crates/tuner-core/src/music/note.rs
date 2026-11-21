//! Математика нот: MIDI, частоты, центы (docs/source/04_dsp.md, 4.3.6).
//!
//! Равномерная темперация, A4 = MIDI 69. Опорная частота A4 задаётся
//! пользователем (400–480 Гц), все формулы параметризованы ею.

/// MIDI-номер опорной ноты A4.
pub const A4_MIDI: u8 = 69;

const LATIN: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

const SOLFEGGIO: [&str; 12] = [
    "До",
    "До#",
    "Ре",
    "Ре#",
    "Ми",
    "Фа",
    "Фа#",
    "Соль",
    "Соль#",
    "Ля",
    "Ля#",
    "Си",
];

/// Целевая частота ноты `midi` (Гц) при опорной частоте `a4`.
///
/// `f_t(m) = a4 · 2^((m − 69)/12)`
pub fn hz(midi: u8, a4: f64) -> f64 {
    a4 * 2f64.powf((f64::from(midi) - f64::from(A4_MIDI)) / 12.0)
}

/// Отклонение частоты `f` от целевой `target` в центах.
///
/// `c = 1200 · log2(f / target)`. Предполагается `f > 0` и `target > 0`.
pub fn cents(f: f64, target: f64) -> f64 {
    1200.0 * (f / target).log2()
}

/// Ближайшая нота равномерной темперации для частоты `f` при опорной `a4`.
///
/// `m = round(69 + 12 · log2(f / a4))`, результат ограничен диапазоном MIDI 0–127.
pub fn nearest_midi(f: f64, a4: f64) -> u8 {
    let m = (f64::from(A4_MIDI) + 12.0 * (f / a4).log2()).round();
    m.clamp(0.0, 127.0) as u8
}

/// Имя ноты (буква, октава) в латинской нотации; C4 — первая октава.
pub fn latin_name(midi: u8) -> (&'static str, i32) {
    (LATIN[usize::from(midi % 12)], i32::from(midi / 12) - 1)
}

/// Имя ноты (слог, октава) в сольфеджио-нотации (До-Ре-Ми).
pub fn solfeggio_name(midi: u8) -> (&'static str, i32) {
    (SOLFEGGIO[usize::from(midi % 12)], i32::from(midi / 12) - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_is_reference() {
        assert!((hz(69, 440.0) - 440.0).abs() < 1e-9);
        assert!((hz(69, 442.0) - 442.0).abs() < 1e-9);
    }

    #[test]
    fn a2_at_442_is_110_50() {
        // Критерий этапа 1: A2 (MIDI 45) при A4 = 442 Гц ровно на две октавы ниже.
        assert!((hz(45, 442.0) - 110.5).abs() < 1e-9);
    }

    #[test]
    fn octave_halves_frequency() {
        assert!((hz(57, 440.0) - 220.0).abs() < 1e-9);
        assert!((hz(81, 440.0) - 880.0).abs() < 1e-9);
    }

    #[test]
    fn cents_zero_on_target() {
        assert!(cents(440.0, 440.0).abs() < 1e-9);
        assert!(cents(110.5, 110.5).abs() < 1e-9);
    }

    #[test]
    fn cents_sign_and_octave() {
        assert!(cents(442.0, 440.0) > 0.0);
        assert!(cents(438.0, 440.0) < 0.0);
        assert!((cents(880.0, 440.0) - 1200.0).abs() < 1e-9);
        // 100 центов = один полутон.
        assert!((cents(hz(70, 440.0), hz(69, 440.0)) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn nearest_midi_rounds() {
        assert_eq!(nearest_midi(440.0, 440.0), 69);
        assert_eq!(nearest_midi(466.16, 440.0), 70);
        assert_eq!(nearest_midi(110.5, 442.0), 45);
        assert_eq!(nearest_midi(30.87, 440.0), 23);
        assert_eq!(nearest_midi(1396.9, 440.0), 89); // F6
    }

    #[test]
    fn nearest_midi_clamps_to_midi_range() {
        assert_eq!(nearest_midi(1.0, 440.0), 0);
        assert_eq!(nearest_midi(100_000.0, 440.0), 127);
    }

    #[test]
    fn note_names() {
        assert_eq!(latin_name(69), ("A", 4));
        assert_eq!(latin_name(40), ("E", 2));
        assert_eq!(latin_name(23), ("B", 0));
        assert_eq!(latin_name(60), ("C", 4));
        assert_eq!(solfeggio_name(69), ("Ля", 4));
        assert_eq!(solfeggio_name(64), ("Ми", 4));
    }

    #[test]
    fn hz_cents_roundtrip() {
        for midi in 21..=96u8 {
            let f = hz(midi, 442.0);
            assert_eq!(nearest_midi(f, 442.0), midi);
            assert!(cents(f, hz(midi, 442.0)).abs() < 1e-9);
        }
    }
}
