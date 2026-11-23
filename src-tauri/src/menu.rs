//! Нативное меню (6.7, 7.2.4): macOS — «О программе Unisono» (версия,
//! Apache-2.0, `unsigned build`, ссылка на PRIVACY.md), «Настройки…» ⌘,,
//! Hide / Hide Others / Show All, «Выход» ⌘Q; Правка для полей ввода.
//! Не-macOS — минимальное меню (Настройки, Выход), без macOS-специфики.

use tauri::menu::{AboutMetadata, MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter};

/// «Настройки…» ⌘,: общий пункт для всех платформ.
fn settings_item(app: &AppHandle) -> tauri::Result<MenuItem<tauri::Wry>> {
    MenuItem::with_id(app, "settings", "Настройки…", true, Some("CmdOrCtrl+,"))
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let settings = settings_item(app)?;

    #[cfg(target_os = "macos")]
    let menu = {
        let about = AboutMetadata {
            name: Some("Unisono".into()),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            copyright: Some("© 2026 Егор Присажный. Apache-2.0".into()),
            license: Some("Apache-2.0".into()),
            // «unsigned build» — не создаём ложного впечатления подписи (7.2.3).
            comments: Some("unsigned build · офлайн, без сбора данных".into()),
            website: Some("https://github.com/eop10/unisono/blob/main/PRIVACY.md".into()),
            website_label: Some("PRIVACY.md".into()),
            ..Default::default()
        };
        let app_menu = SubmenuBuilder::new(app, "Unisono")
            .about(Some(about))
            .separator()
            .item(&settings)
            .separator()
            .hide()
            .hide_others()
            .show_all()
            .separator()
            .quit()
            .build()?;
        // Правка: копирование/вставка/выделение в полях ввода (редактор строя).
        let edit = SubmenuBuilder::new(app, "Правка")
            .cut()
            .copy()
            .paste()
            .select_all()
            .build()?;
        MenuBuilder::new(app).item(&app_menu).item(&edit).build()?
    };

    #[cfg(not(target_os = "macos"))]
    let menu = MenuBuilder::new(app)
        .item(&settings)
        .separator()
        .quit()
        .build()?;

    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        if event.id().as_ref() == "settings" {
            // ⌘, из меню: лист настроек открывает UI (6.7); аудио не трогаем.
            let _ = app.emit("menu-settings", ());
        }
    });
    Ok(())
}
