//! Встроенные строи (docs/source/05_data.md, 5.5) и правило унисонов (D-7).
//!
//! MIDI-ноты струн в порядке струн инструмента (от низкой к высокой;
//! у укулеле High-G первая струна G4 — реентрантный строй, порядок сохраняется).
//! Частоты не хранятся: пересчитываются из MIDI при смене опорной A4.

use super::instruments::Localized;

/// Имя строя: локализованное для встроенных, пользовательская строка для своих.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum LocalizedName {
    BuiltIn(Localized),
    Custom(String),
}

/// Строй: привязан к инструменту, струны — MIDI-ноты.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Tuning {
    pub id: String,
    pub instrument_id: String,
    pub name: LocalizedName,
    /// MIDI струн в порядке струн; пусто для хроматического режима.
    pub strings: Vec<u8>,
    pub built_in: bool,
}

impl Tuning {
    /// Индексы всех струн с нотой `midi`.
    ///
    /// Правило унисонов (ФТ-15, D-7): дубли нот разрешены — балалайка-прима
    /// E4+E4 отдаёт пару индексов, а не одну случайную струну. Пустой список,
    /// если такой ноты в строе нет.
    pub fn string_indices(&self, midi: u8) -> Vec<usize> {
        self.strings
            .iter()
            .enumerate()
            .filter(|(_, &m)| m == midi)
            .map(|(i, _)| i)
            .collect()
    }
}

/// Определение встроенного строя для таблицы 5.5.
pub(crate) struct TuningDef {
    pub id: &'static str,
    pub instrument_id: &'static str,
    pub ru: &'static str,
    pub en: &'static str,
    pub strings: &'static [u8],
}

/// 23 встроенных строя из 5.5.
pub(crate) const TUNING_DEFS: &[TuningDef] = &[
    // Гитара (9)
    TuningDef {
        id: "standard_e",
        instrument_id: "guitar",
        ru: "Стандарт E",
        en: "Standard E",
        strings: &[40, 45, 50, 55, 59, 64],
    },
    TuningDef {
        id: "drop_d",
        instrument_id: "guitar",
        ru: "Drop D",
        en: "Drop D",
        strings: &[38, 45, 50, 55, 59, 64],
    },
    TuningDef {
        id: "eb",
        instrument_id: "guitar",
        ru: "Eb стандарт",
        en: "Eb standard",
        strings: &[39, 44, 49, 54, 58, 63],
    },
    TuningDef {
        id: "d_standard",
        instrument_id: "guitar",
        ru: "D стандарт",
        en: "D standard",
        strings: &[38, 43, 48, 53, 57, 62],
    },
    TuningDef {
        id: "drop_c",
        instrument_id: "guitar",
        ru: "Drop C",
        en: "Drop C",
        strings: &[36, 43, 48, 53, 57, 62],
    },
    TuningDef {
        id: "dadgad",
        instrument_id: "guitar",
        ru: "DADGAD",
        en: "DADGAD",
        strings: &[38, 45, 50, 55, 57, 62],
    },
    TuningDef {
        id: "open_g",
        instrument_id: "guitar",
        ru: "Open G",
        en: "Open G",
        strings: &[38, 43, 50, 55, 59, 62],
    },
    TuningDef {
        id: "open_d",
        instrument_id: "guitar",
        ru: "Open D",
        en: "Open D",
        strings: &[38, 45, 50, 54, 57, 62],
    },
    TuningDef {
        id: "guitar_7",
        instrument_id: "guitar",
        ru: "7-струнная",
        en: "7-string",
        strings: &[35, 40, 45, 50, 55, 59, 64],
    },
    // Бас-гитара (2)
    TuningDef {
        id: "bass4",
        instrument_id: "bass",
        ru: "Бас 4",
        en: "Bass 4",
        strings: &[28, 33, 38, 43],
    },
    TuningDef {
        id: "bass5",
        instrument_id: "bass",
        ru: "Бас 5",
        en: "Bass 5",
        strings: &[23, 28, 33, 38, 43],
    },
    // Укулеле (2)
    TuningDef {
        id: "uke_high_g",
        instrument_id: "ukulele",
        ru: "High-G",
        en: "High-G",
        strings: &[67, 60, 64, 69],
    },
    TuningDef {
        id: "uke_low_g",
        instrument_id: "ukulele",
        ru: "Low-G",
        en: "Low-G",
        strings: &[55, 60, 64, 69],
    },
    // Смычковые (4) — ID глобально уникальны (5.5)
    TuningDef {
        id: "violin_std",
        instrument_id: "violin",
        ru: "Стандарт",
        en: "Standard",
        strings: &[55, 62, 69, 76],
    },
    TuningDef {
        id: "viola_std",
        instrument_id: "viola",
        ru: "Стандарт",
        en: "Standard",
        strings: &[48, 55, 62, 69],
    },
    TuningDef {
        id: "cello_std",
        instrument_id: "cello",
        ru: "Стандарт",
        en: "Standard",
        strings: &[36, 43, 50, 57],
    },
    TuningDef {
        id: "contrabass_std",
        instrument_id: "contrabass",
        ru: "Стандарт",
        en: "Standard",
        strings: &[28, 33, 38, 43],
    },
    // Балалайка (1) — две струны-унисона E4
    TuningDef {
        id: "balalaika_prima",
        instrument_id: "balalaika",
        ru: "Прима",
        en: "Prima",
        strings: &[64, 64, 69],
    },
    // Домра (2)
    TuningDef {
        id: "domra_3",
        instrument_id: "domra",
        ru: "3-струнная (квартовый)",
        en: "3-string (quartal)",
        strings: &[64, 69, 74],
    },
    TuningDef {
        id: "domra_4",
        instrument_id: "domra",
        ru: "4-струнная (квинтовый)",
        en: "4-string (quintal)",
        strings: &[55, 62, 69, 76],
    },
    // Мандолина (1) — 4 курса, пара настраивается как одна струна
    TuningDef {
        id: "mandolin_std",
        instrument_id: "mandolin",
        ru: "Стандарт",
        en: "Standard",
        strings: &[55, 62, 69, 76],
    },
    // Банджо (2)
    TuningDef {
        id: "banjo_5",
        instrument_id: "banjo",
        ru: "5-струнное, Open G",
        en: "5-string, Open G",
        strings: &[50, 55, 59, 62, 67],
    },
    TuningDef {
        id: "banjo_tenor",
        instrument_id: "banjo",
        ru: "Тенор C-G-D-A",
        en: "Tenor C-G-D-A",
        strings: &[48, 55, 62, 69],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn balalaika() -> Tuning {
        Tuning {
            id: "balalaika_prima".into(),
            instrument_id: "balalaika".into(),
            name: LocalizedName::BuiltIn(Localized {
                ru: "Прима",
                en: "Prima",
            }),
            strings: vec![64, 64, 69],
            built_in: true,
        }
    }

    #[test]
    fn unison_pair_returns_both_indices() {
        let t = balalaika();
        assert_eq!(t.string_indices(64), vec![0, 1]);
        assert_eq!(t.string_indices(69), vec![2]);
        assert!(t.string_indices(60).is_empty());
    }

    #[test]
    fn defs_count_is_23() {
        assert_eq!(TUNING_DEFS.len(), 23);
    }

    #[test]
    fn defs_reference_known_instruments() {
        for def in TUNING_DEFS {
            assert!(super::super::instruments::instrument(def.instrument_id).is_some());
            assert!(!def.strings.is_empty());
        }
    }
}
