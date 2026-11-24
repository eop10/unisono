mod bridge;
mod commands;
// Нативное меню — desktop-платформы (Tauri не собирает menu под Android).
#[cfg(desktop)]
mod menu;

use tauri::Manager;

#[cfg(target_os = "android")]
pub struct MicHandle(pub tauri::plugin::PluginHandle<tauri::Wry>);
#[cfg(not(target_os = "android"))]
pub struct MicHandle;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mic_plugin = tauri::plugin::Builder::<tauri::Wry>::new("mic")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api.register_android_plugin("app.unisono.tuner", "MicPlugin")?;
                app.manage(MicHandle(handle));
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = api;
                app.manage(MicHandle);
            }
            Ok(())
        })
        .build();

    tauri::Builder::default()
        .plugin(mic_plugin)
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
            commands::platform,
            commands::has_mic_permission,
            commands::request_mic,
            commands::keep_screen_on,
            commands::vibrate,
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
            // Нативное меню (6.7): до запуска окна, ошибки не критичны.
            #[cfg(desktop)]
            if let Err(e) = menu::build(app.handle()) {
                log::warn!("меню не собрано: {e}");
            }
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
