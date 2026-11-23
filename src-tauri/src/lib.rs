mod bridge;
mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(bridge::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_catalog,
            commands::start_listening,
            commands::stop_listening,
            commands::set_reference,
            commands::select_instrument,
            commands::select_tuning,
            commands::select_string,
            commands::set_tolerance,
            commands::set_sensitivity,
            commands::set_low_range,
            commands::get_settings,
            commands::save_settings,
            commands::get_presets,
            commands::save_preset,
            commands::delete_preset,
        ])
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                // Окно закрыто — захват останавливаем, даже если UI не успел
                // вызвать stop_listening (3.6: окно живёт отдельно от аудио).
                let state = window.app_handle().state::<bridge::AppState>();
                state.stop();
            }
        })
        .setup(|app| {
            // Каталог конфигурации: настройки и пресеты переживают перезапуск (5.2).
            match app.path().app_config_dir() {
                Ok(dir) => {
                    let state = app.state::<bridge::AppState>();
                    match tuner_storage::Store::open(&dir) {
                        Ok(store) => state.set_store(store),
                        Err(e) => log::warn!("хранилище недоступно: {e}"),
                    }
                }
                Err(e) => log::warn!("каталог конфигурации недоступен: {e}"),
            }
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
