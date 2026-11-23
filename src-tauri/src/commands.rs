//! Команды Tauri (3.5): каталог и управление захватом/движком.
//! Тонкие обёртки над [`AppState`]; центы и ноты считает движок, не bridge.

use tauri::ipc::Channel;
use tauri::State;
use tuner_audio::AudioError;
use tuner_core::engine::{Sensitivity, TunerState};
use tuner_core::music::InstrumentRegistry;

use crate::bridge::{send_states_to, AppState, TuningSpec};

/// Каталог для UI: инструменты и встроенные строи из ядра, без дублей в TS.
/// Пользовательские строи приезжают из tuner-storage (4B), не отсюда.
#[tauri::command]
pub fn get_catalog() -> serde_json::Value {
    let reg = InstrumentRegistry::new();
    let tunings: Vec<_> = reg
        .instruments()
        .iter()
        .flat_map(|i| reg.tunings_for(i.id))
        .collect();
    serde_json::json!({
        "instruments": tuner_core::music::INSTRUMENTS,
        "tunings": tunings,
    })
}

/// Начать прослушивание: состояния уходят в `channel` (30–60 Гц, 3.6),
/// сырое аудио через IPC не передаётся. Ошибка устройства — `AudioError`
/// в UI-стор превращается в `error`/`nopermission`.
#[tauri::command]
pub fn start_listening(
    channel: Channel<TunerState>,
    state: State<'_, AppState>,
) -> Result<(), AudioError> {
    state.start_capture(send_states_to(channel))
}

/// Остановить прослушивание: cpal и поток анализа (включая закрытие окна).
#[tauri::command]
pub fn stop_listening(state: State<'_, AppState>) {
    state.stop();
}

/// Опорная частота A4, Гц (400–480): движок пересчитывает цели без скачка.
#[tauri::command]
pub fn set_reference(hz: f64, state: State<'_, AppState>) {
    state.set_reference(hz);
}

/// Инструмент: первый строй из реестра; хроматика убирает струны (D-6).
#[tauri::command]
pub fn select_instrument(id: String, state: State<'_, AppState>) {
    state.select_instrument(&id);
}

/// Строй целиком: и встроенный, и пользовательский (UI всегда имеет объект).
#[tauri::command]
pub fn select_tuning(tuning: TuningSpec, state: State<'_, AppState>) {
    state.select_tuning(tuning);
}

/// Ручная струна (ФТ-15); `null` — вернуться к автоопределению.
#[tauri::command]
pub fn select_string(index: Option<usize>, state: State<'_, AppState>) {
    state.select_string(index);
}

/// Допуск «в строе» в центах (2/3/5).
#[tauri::command]
pub fn set_tolerance(cents: f32, state: State<'_, AppState>) {
    state.set_tolerance(cents);
}

/// Чувствительность входного гейта.
#[tauri::command]
pub fn set_sensitivity(sensitivity: Sensitivity, state: State<'_, AppState>) {
    state.set_sensitivity(sensitivity);
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
