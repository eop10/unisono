//! Команды Tauri: каталог инструментов и строёв из ядра (без дублирования в TS).

use tuner_core::music::{InstrumentRegistry, INSTRUMENTS};

/// Каталог для UI: инструменты (id, имена ru/en, kind) и встроенные строи
/// (id, instrument_id, name, strings, built_in). Пользовательские строи
/// добавляются на этапе хранения (tuner-storage), не здесь.
#[tauri::command]
pub fn get_catalog() -> serde_json::Value {
    let reg = InstrumentRegistry::new();
    let tunings: Vec<_> = reg
        .instruments()
        .iter()
        .flat_map(|i| reg.tunings_for(i.id))
        .collect();
    serde_json::json!({
        "instruments": INSTRUMENTS,
        "tunings": tunings,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn catalog_has_12_instruments_and_23_tunings() {
        let v = super::get_catalog();
        assert_eq!(v["instruments"].as_array().unwrap().len(), 12);
        assert_eq!(v["tunings"].as_array().unwrap().len(), 23);
        let violin = &v["tunings"][13];
        assert_eq!(violin["id"], "violin_std");
        assert_eq!(violin["name"]["BuiltIn"]["ru"], "Стандарт");
        assert_eq!(violin["strings"][0], 55);
        assert_eq!(violin["built_in"], true);
    }
}
