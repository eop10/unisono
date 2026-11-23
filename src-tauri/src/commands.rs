//! Команды Tauri (3.5): каталог и управление захватом/движком.
//! Тонкие обёртки над [`AppState`]; центы и ноты считает движок, не bridge.

use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tuner_audio::AudioError;
use tuner_core::engine::{Sensitivity, TunerState};
use tuner_core::music::InstrumentRegistry;

use crate::bridge::{AppState, TuningSpec};

/// Каталог для UI: инструменты и встроенные строи из ядра, без дублей в TS.
/// Пользовательские строи приезжают из tuner-storage (4B), не отсюда.
#[tauri::command]
pub fn get_catalog() -> serde_json::Value {
    log::info!("get_catalog");
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
/// Не блокирует главный поток (3.3): TCC-запрос ожидает в `spawn_blocking`.
#[tauri::command]
pub async fn start_listening(
    app: AppHandle,
    channel: Channel<TunerState>,
) -> Result<(), AudioError> {
    log::info!("start_listening");
    let sink = crate::bridge::channel_sink(channel);
    let result =
        tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().start_capture(sink))
            .await
            .map_err(|e| AudioError::Stream(format!("поток запуска упал: {e}")))?;
    if let Err(e) = &result {
        log::warn!("start_listening не удался: {e}");
    }
    result
}

/// Остановить прослушивание: cpal и поток анализа (включая закрытие окна).
/// Выполняется в `spawn_blocking`, чтобы не блокировать главный поток.
#[tauri::command]
pub async fn stop_listening(app: AppHandle) {
    let _ = tauri::async_runtime::spawn_blocking(move || {
        app.state::<AppState>().stop();
    })
    .await;
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

/// «Низкий диапазон» (5.3): окно 16384 в хроматическом режиме.
#[tauri::command]
pub fn set_low_range(low_range: bool, state: State<'_, AppState>) {
    state.set_low_range(low_range);
}

/// Настройки, переживающие перезапуск (5.2–5.3): файл или дефолт.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> crate::bridge::SettingsDto {
    log::info!("get_settings");
    state.settings_dto()
}

/// Сохранение настроек: ошибки хранятся в логе, UI не роняют.
#[tauri::command]
pub fn save_settings(settings: crate::bridge::SettingsDto, state: State<'_, AppState>) {
    state.save_settings_dto(settings);
}

/// Пользовательские строи из файла (5.6).
#[tauri::command]
pub fn get_presets(state: State<'_, AppState>) -> Vec<crate::bridge::PresetDto> {
    state.presets_dto()
}

/// Создать/обновить пользовательский строй; лимит 20 проверяется в Rust.
#[tauri::command]
pub fn save_preset(preset: crate::bridge::PresetDto, state: State<'_, AppState>) {
    state.save_preset_dto(preset);
}

/// Удалить пользовательский строй по ID.
#[tauri::command]
pub fn delete_preset(id: String, state: State<'_, AppState>) {
    state.delete_preset(&id);
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
