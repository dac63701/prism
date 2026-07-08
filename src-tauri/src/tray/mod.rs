//! System tray integration — background operation and quick actions.
//!
//! Builds a tray icon with context menu for:
//! - Save Clip
//! - Open Library
//! - Settings
//! - Separator
//! - Quit

use tauri::{
    AppHandle, Emitter, Manager, Runtime,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

/// Build and attach the system tray with context menu.
pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let save_clip =
        MenuItemBuilder::with_id("save_clip", "Save Clip").accelerator("CmdOrCtrl+Shift+X").build(app)?;
    let open_library =
        MenuItemBuilder::with_id("open_library", "Open Library").accelerator("CmdOrCtrl+Shift+L").build(app)?;
    let settings_item =
        MenuItemBuilder::with_id("open_settings", "Settings").accelerator("CmdOrCtrl+,").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit Prism").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&save_clip)
        .item(&open_library)
        .item(&settings_item)
        .separator()
        .item(&quit)
        .build()?;

    TrayIconBuilder::new()
        .tooltip("Prism — Game Clipping")
        .icon(
            app.default_window_icon()
                .cloned()
                .unwrap_or_else(|| {
                    // 1x1 transparent pixel as safety fallback
                    tauri::image::Image::new(&[0, 0, 0, 0], 1, 1)
                })
        )
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "save_clip" => {
                let _ = app.emit("menu-action", "save_clip");
            }
            "open_library" => {
                let _ = app.emit("menu-action", "open_library");
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "open_settings" => {
                let _ = app.emit("menu-action", "open_settings");
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
