//! Реестр инструментов и строёв: списки, нижняя нота, выбор окна анализа.
//!
//! Окно выбирается по нижней ноте активного строя (docs/source/04_dsp.md, 4.2.3;
//! D-3, ADR-006), а не глобально. Границы окна — ADR-009.

use super::instruments::{Instrument, INSTRUMENTS};
use super::note;
use super::tunings::{LocalizedName, Tuning, TUNING_DEFS};

/// Нижняя нота гитары E2 (MIDI 40, 82,4 Гц): от неё и выше — окно 4096.
const E2_MIDI: u8 = 40;
/// B1 (MIDI 35, 61,7 Гц): нижняя граница окна 8192 (τ_max ≈ 780 при 48 кГц
/// покрывает частоты до ≈ 61,5 Гц). Ниже B1 — окно 16384.
const B1_MIDI: u8 = 35;

/// Реестр встроенных инструментов и строёв.
///
/// Пользовательские строи (этап 3, `tuner-storage`) добавляются поверх
/// встроенных и всегда привязаны к инструменту.
#[derive(Clone, Debug, Default)]
pub struct InstrumentRegistry {
    tunings: Vec<Tuning>,
}

impl InstrumentRegistry {
    /// Реестр со всеми встроенными строями из 5.5.
    pub fn new() -> Self {
        let tunings = TUNING_DEFS
            .iter()
            .map(|def| Tuning {
                id: def.id.to_string(),
                instrument_id: def.instrument_id.to_string(),
                name: LocalizedName::BuiltIn(super::instruments::Localized {
                    ru: def.ru,
                    en: def.en,
                }),
                strings: def.strings.to_vec(),
                built_in: true,
            })
            .collect();
        Self { tunings }
    }

    /// Все инструменты в порядке списка UI.
    pub fn instruments(&self) -> &'static [Instrument] {
        INSTRUMENTS
    }

    /// Встроенные строи инструмента; для хроматики — пустой список.
    pub fn tunings_for(&self, instrument_id: &str) -> Vec<&Tuning> {
        self.tunings
            .iter()
            .filter(|t| t.instrument_id == instrument_id)
            .collect()
    }

    /// Строй по паре (инструмент, строй); чужой строй не находится (D-7).
    pub fn tuning(&self, instrument_id: &str, tuning_id: &str) -> Option<&Tuning> {
        self.tunings
            .iter()
            .find(|t| t.instrument_id == instrument_id && t.id == tuning_id)
    }

    /// Нижняя нота строя (MIDI); `None` для хроматики без струн.
    ///
    /// Минимум, а не первая струна: у укулеле High-G порядок струн
    /// реентрантный (G4 C4 E4 A4).
    pub fn lowest_note(tuning: &Tuning) -> Option<u8> {
        tuning.strings.iter().copied().min()
    }

    /// Окно анализа (отсчёты) по нижней ноте строя: 4096 / 8192 / 16384.
    ///
    /// Для строя без струн (хроматика) — базовые 4096; режим «Низкий диапазон»
    /// переключает хроматику на 16384 через [`Self::chromatic_window`].
    pub fn window_size(tuning: &Tuning) -> u32 {
        match Self::lowest_note(tuning) {
            Some(m) if m >= E2_MIDI => 4096,
            Some(m) if m >= B1_MIDI => 8192,
            Some(_) => 16384,
            None => 4096,
        }
    }

    /// Окно хроматического режима: 4096, с «Низким диапазоном» — 16384 (4.2.3).
    pub fn chromatic_window(low_range: bool) -> u32 {
        if low_range {
            16384
        } else {
            4096
        }
    }

    /// Целевая частота струны `index` при опорной `a4` (Гц).
    pub fn string_hz(tuning: &Tuning, index: usize, a4: f64) -> Option<f64> {
        tuning.strings.get(index).map(|&m| note::hz(m, a4))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Все 23 строя из 5.5: (инструмент, строй, MIDI струн).
    const EXPECTED: &[(&str, &str, &[u8])] = &[
        ("guitar", "standard_e", &[40, 45, 50, 55, 59, 64]),
        ("guitar", "drop_d", &[38, 45, 50, 55, 59, 64]),
        ("guitar", "eb", &[39, 44, 49, 54, 58, 63]),
        ("guitar", "d_standard", &[38, 43, 48, 53, 57, 62]),
        ("guitar", "drop_c", &[36, 43, 48, 53, 57, 62]),
        ("guitar", "dadgad", &[38, 45, 50, 55, 57, 62]),
        ("guitar", "open_g", &[38, 43, 50, 55, 59, 62]),
        ("guitar", "open_d", &[38, 45, 50, 54, 57, 62]),
        ("guitar", "guitar_7", &[35, 40, 45, 50, 55, 59, 64]),
        ("bass", "bass4", &[28, 33, 38, 43]),
        ("bass", "bass5", &[23, 28, 33, 38, 43]),
        ("ukulele", "uke_high_g", &[67, 60, 64, 69]),
        ("ukulele", "uke_low_g", &[55, 60, 64, 69]),
        ("violin", "violin_std", &[55, 62, 69, 76]),
        ("viola", "viola_std", &[48, 55, 62, 69]),
        ("cello", "cello_std", &[36, 43, 50, 57]),
        ("contrabass", "contrabass_std", &[28, 33, 38, 43]),
        ("balalaika", "balalaika_prima", &[64, 64, 69]),
        ("domra", "domra_3", &[64, 69, 74]),
        ("domra", "domra_4", &[55, 62, 69, 76]),
        ("mandolin", "mandolin_std", &[55, 62, 69, 76]),
        ("banjo", "banjo_5", &[50, 55, 59, 62, 67]),
        ("banjo", "banjo_tenor", &[48, 55, 62, 69]),
    ];

    #[test]
    fn all_23_tunings_match_spec() {
        let reg = InstrumentRegistry::new();
        assert_eq!(EXPECTED.len(), 23);
        let total: usize = reg
            .instruments()
            .iter()
            .map(|i| reg.tunings_for(i.id).len())
            .sum();
        assert_eq!(total, 23);
        for &(instrument, tuning, strings) in EXPECTED {
            let t = reg
                .tuning(instrument, tuning)
                .unwrap_or_else(|| panic!("{instrument}/{tuning} отсутствует"));
            assert_eq!(t.strings, strings, "{instrument}/{tuning}");
            assert!(t.built_in);
        }
    }

    #[test]
    fn tunings_are_scoped_to_instrument() {
        let reg = InstrumentRegistry::new();
        // Строй виолончели не виден у скрипки: ID глобально уникальны (5.5),
        // поиск всё равно выполняется парой (инструмент, строй).
        assert!(reg.tuning("violin", "violin_std").is_some());
        assert!(reg.tuning("violin", "cello_std").is_none());
        assert!(reg.tuning("cello", "violin_std").is_none());
        assert_eq!(
            reg.tuning("violin", "violin_std").unwrap().strings,
            vec![55, 62, 69, 76]
        );
        assert_eq!(
            reg.tuning("cello", "cello_std").unwrap().strings,
            vec![36, 43, 50, 57]
        );
        assert!(reg.tuning("guitar", "bass4").is_none());
        assert!(reg.tunings_for("chromatic").is_empty());
    }

    #[test]
    fn builtin_tuning_ids_are_globally_unique() {
        let reg = InstrumentRegistry::new();
        let mut ids: Vec<&str> = reg
            .instruments()
            .iter()
            .flat_map(|i| reg.tunings_for(i.id))
            .map(|t| t.id.as_str())
            .collect();
        assert_eq!(ids.len(), 23);
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            ids.len(),
            23,
            "ID встроенных строёв должны быть глобально уникальны"
        );
        assert!(!ids.contains(&"standard"));
    }

    #[test]
    fn builtin_strings_count_and_midi_range() {
        let reg = InstrumentRegistry::new();
        for instrument in reg.instruments() {
            for t in reg.tunings_for(instrument.id) {
                assert!(
                    (1..=8).contains(&t.strings.len()),
                    "{}/{}: {} струн",
                    instrument.id,
                    t.id,
                    t.strings.len()
                );
                for &m in &t.strings {
                    assert!(
                        (21..=96).contains(&m),
                        "{}/{}: MIDI {m} вне 21–96",
                        instrument.id,
                        t.id
                    );
                }
            }
        }
    }

    #[test]
    fn target_frequencies_within_range_at_all_references() {
        let reg = InstrumentRegistry::new();
        // Диапазон 30–1400 Гц (ФТ-02) задан при A4 = 440; при смене калибровки
        // границы масштабируются пропорционально. Краевой случай: B0 баса-5
        // при A4 = 400 = 28,06 Гц — ниже номинальных 30 Гц, но внутри
        // масштабированной границы; детектор видит её, так как tau_max
        // считается от фактической нижней цели (0.94 · f_min).
        for a4 in [400.0, 440.0, 480.0] {
            let lo = 30.0 * a4 / 440.0;
            let hi = 1400.0 * a4 / 440.0;
            for instrument in reg.instruments() {
                for t in reg.tunings_for(instrument.id) {
                    for &m in &t.strings {
                        let f = note::hz(m, a4);
                        assert!(
                            f >= lo && f <= hi,
                            "{}/{}: MIDI {m} при A4 {a4} = {f} Гц вне [{lo}, {hi}]",
                            instrument.id,
                            t.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn target_frequencies_at_440() {
        let reg = InstrumentRegistry::new();
        let cases: &[(&str, &str, usize, f64)] = &[
            ("guitar", "standard_e", 0, 82.41),          // E2
            ("guitar", "standard_e", 5, 329.63),         // E4
            ("violin", "violin_std", 0, 196.0),          // G3
            ("violin", "violin_std", 3, 659.26),         // E5
            ("cello", "cello_std", 0, 65.41),            // C2
            ("bass", "bass5", 0, 30.87),                 // B0
            ("contrabass", "contrabass_std", 0, 41.2),   // E1
            ("balalaika", "balalaika_prima", 0, 329.63), // E4
            ("balalaika", "balalaika_prima", 2, 440.0),  // A4
        ];
        for &(instrument, tuning, index, expected) in cases {
            let t = reg.tuning(instrument, tuning).unwrap();
            let f = InstrumentRegistry::string_hz(t, index, 440.0).unwrap();
            assert!(
                (f - expected).abs() < 0.01,
                "{instrument}/{tuning}[{index}]: {f} != {expected}"
            );
        }
        assert!(InstrumentRegistry::string_hz(
            reg.tuning("guitar", "standard_e").unwrap(),
            6,
            440.0
        )
        .is_none());
    }

    #[test]
    fn targets_recompute_with_a4() {
        let reg = InstrumentRegistry::new();
        let violin = reg.tuning("violin", "violin_std").unwrap();
        // A4 = 442: струна A4 скрипки ровно 442, остальные масштабируются на 442/440.
        assert!((InstrumentRegistry::string_hz(violin, 2, 442.0).unwrap() - 442.0).abs() < 1e-9);
        let ratio = InstrumentRegistry::string_hz(violin, 0, 442.0).unwrap()
            / InstrumentRegistry::string_hz(violin, 0, 440.0).unwrap();
        assert!((ratio - 442.0 / 440.0).abs() < 1e-12);
    }

    #[test]
    fn window_size_by_lowest_note() {
        let reg = InstrumentRegistry::new();
        let cases: &[(&str, &str, u32)] = &[
            ("guitar", "standard_e", 4096),  // E2
            ("violin", "violin_std", 4096),  // G3
            ("viola", "viola_std", 4096),    // C3
            ("ukulele", "uke_high_g", 4096), // min C4
            ("ukulele", "uke_low_g", 4096),  // G3
            ("mandolin", "mandolin_std", 4096),
            ("domra", "domra_3", 4096),
            ("domra", "domra_4", 4096),
            ("banjo", "banjo_5", 4096),              // D3
            ("banjo", "banjo_tenor", 4096),          // C3
            ("guitar", "drop_d", 8192),              // D2 = 73,4 Гц
            ("guitar", "eb", 8192),                  // D#2
            ("guitar", "d_standard", 8192),          // D2
            ("guitar", "dadgad", 8192),              // D2
            ("guitar", "open_g", 8192),              // D2
            ("guitar", "open_d", 8192),              // D2
            ("cello", "cello_std", 8192),            // C2
            ("guitar", "drop_c", 8192),              // C2
            ("guitar", "guitar_7", 8192),            // B1 (ADR-009)
            ("bass", "bass4", 16384),                // E1
            ("bass", "bass5", 16384),                // B0
            ("contrabass", "contrabass_std", 16384), // E1
        ];
        for &(instrument, tuning, window) in cases {
            let t = reg.tuning(instrument, tuning).unwrap();
            assert_eq!(
                InstrumentRegistry::window_size(t),
                window,
                "{instrument}/{tuning}"
            );
        }
    }

    #[test]
    fn window_boundaries_are_midi_40_and_35() {
        let mut t = Tuning {
            id: "x".into(),
            instrument_id: "guitar".into(),
            name: LocalizedName::Custom("x".into()),
            strings: vec![40],
            built_in: false,
        };
        assert_eq!(InstrumentRegistry::window_size(&t), 4096); // E2
        t.strings = vec![39];
        assert_eq!(InstrumentRegistry::window_size(&t), 8192); // D#2
        t.strings = vec![35];
        assert_eq!(InstrumentRegistry::window_size(&t), 8192); // B1
        t.strings = vec![34];
        assert_eq!(InstrumentRegistry::window_size(&t), 16384); // A#1
        t.strings = vec![23];
        assert_eq!(InstrumentRegistry::window_size(&t), 16384); // B0
        t.strings = vec![21];
        assert_eq!(InstrumentRegistry::window_size(&t), 16384); // A0, ниже диапазона
    }

    #[test]
    fn chromatic_window_switch() {
        assert_eq!(InstrumentRegistry::chromatic_window(false), 4096);
        assert_eq!(InstrumentRegistry::chromatic_window(true), 16384);
        let empty = Tuning {
            id: String::new(),
            instrument_id: "chromatic".into(),
            name: LocalizedName::Custom(String::new()),
            strings: vec![],
            built_in: false,
        };
        assert_eq!(InstrumentRegistry::lowest_note(&empty), None);
        assert_eq!(InstrumentRegistry::window_size(&empty), 4096);
    }

    #[test]
    fn lowest_note_is_min_not_first_string() {
        let reg = InstrumentRegistry::new();
        let high_g = reg.tuning("ukulele", "uke_high_g").unwrap();
        assert_eq!(high_g.strings[0], 67);
        assert_eq!(InstrumentRegistry::lowest_note(high_g), Some(60));
    }

    #[test]
    fn balalaika_unison_pair() {
        let reg = InstrumentRegistry::new();
        let t = reg.tuning("balalaika", "balalaika_prima").unwrap();
        assert_eq!(t.string_indices(64), vec![0, 1]);
        assert_eq!(t.string_indices(69), vec![2]);
        // Целевые частоты пары одинаковы.
        assert_eq!(
            InstrumentRegistry::string_hz(t, 0, 442.0),
            InstrumentRegistry::string_hz(t, 1, 442.0)
        );
    }
}
