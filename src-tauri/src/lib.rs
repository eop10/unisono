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
