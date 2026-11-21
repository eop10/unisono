//! Встроенные инструменты (docs/source/05_data.md, 5.4–5.5).
//!
//! 11 струнных инструментов + хроматический режим как позиция списка (D-5, D-6).
//! Данные — константы: добавление инструмента не меняет UI и DSP.

/// Локализованное название (НФТ-12: RU/EN).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Localized {
    pub ru: &'static str,
    pub en: &'static str,
}

/// Тип инструмента: струнный со строями или хроматический без струн.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum InstrumentKind {
    Stringed,
    Chromatic,
}

/// Инструмент: ID, локализованное имя, тип.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Instrument {
    pub id: &'static str,
    pub name: Localized,
    pub kind: InstrumentKind,
}

const fn stringed(id: &'static str, ru: &'static str, en: &'static str) -> Instrument {
    Instrument {
        id,
        name: Localized { ru, en },
        kind: InstrumentKind::Stringed,
    }
}

/// Все инструменты в порядке списка UI; хроматический — последний.
pub const INSTRUMENTS: &[Instrument] = &[
    stringed("guitar", "Гитара", "Guitar"),
    stringed("bass", "Бас-гитара", "Bass guitar"),
    stringed("ukulele", "Укулеле", "Ukulele"),
    stringed("violin", "Скрипка", "Violin"),
    stringed("viola", "Альт", "Viola"),
    stringed("cello", "Виолончель", "Cello"),
    stringed("contrabass", "Контрабас", "Double bass"),
    stringed("balalaika", "Балалайка", "Balalaika"),
    stringed("domra", "Домра", "Domra"),
    stringed("mandolin", "Мандолина", "Mandolin"),
    stringed("banjo", "Банджо", "Banjo"),
    Instrument {
        id: "chromatic",
        name: Localized {
            ru: "Хроматический",
            en: "Chromatic",
        },
        kind: InstrumentKind::Chromatic,
    },
];

/// Инструмент по ID (`None`, если ID неизвестен).
pub fn instrument(id: &str) -> Option<&'static Instrument> {
    INSTRUMENTS.iter().find(|i| i.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twelve_instruments_eleven_stringed() {
        assert_eq!(INSTRUMENTS.len(), 12);
        assert_eq!(
            INSTRUMENTS
                .iter()
                .filter(|i| i.kind == InstrumentKind::Stringed)
                .count(),
            11
        );
    }

    #[test]
    fn chromatic_is_last_and_stringless() {
        let chromatic = instrument("chromatic").unwrap();
        assert_eq!(chromatic.kind, InstrumentKind::Chromatic);
        assert_eq!(INSTRUMENTS.last().unwrap().id, "chromatic");
    }

    #[test]
    fn lookup_by_id() {
        assert_eq!(instrument("guitar").unwrap().name.ru, "Гитара");
        assert_eq!(instrument("violin").unwrap().name.en, "Violin");
        assert!(instrument("theremin").is_none());
    }

    #[test]
    fn ids_are_unique() {
        for (i, a) in INSTRUMENTS.iter().enumerate() {
            for b in &INSTRUMENTS[i + 1..] {
                assert_ne!(a.id, b.id);
            }
        }
    }
}
