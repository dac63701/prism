//! Global hotkey registration and handling.
//!
//! Wraps `tauri-plugin-global-shortcut` with a manager that reads
//! hotkey bindings from settings and keeps them in sync.

use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent};

use crate::settings::config::HotkeySettings;

/// Parse a hotkey string like "Cmd+Shift+X" into a `Shortcut`.
pub fn parse_hotkey(hk: &str) -> Result<Shortcut, String> {
    hk.parse::<Shortcut>()
        .map_err(|e| format!("Failed to parse hotkey '{hk}': {e}"))
}

/// Register or re-register all hotkeys from settings.
/// Unregisters any previously registered shortcuts first.
pub fn register_hotkeys<R: Runtime>(
    app: &AppHandle<R>,
    settings: &HotkeySettings,
) -> Result<(), String> {
    let bindings = [
        (&settings.save_clip, "save_clip"),
        (&settings.toggle_recording, "toggle_recording"),
        (&settings.open_library, "open_library"),
    ];

    // Validate first so a bad shortcut does not clear the previous bindings.
    for (hk_str, _action) in &bindings {
        if hk_str.is_empty() {
            continue;
        }
        // Validate by parsing
        let _ = parse_hotkey(hk_str).map_err(|e| format!("Invalid hotkey '{hk_str}': {e}"))?;
    }

    // Safe to replace the old bindings now.
    let _ = app.global_shortcut().unregister_all();

    for (hk_str, action) in &bindings {
        if hk_str.is_empty() {
            continue;
        }

        let act = *action;
        app.global_shortcut()
            .on_shortcut(
                hk_str.as_str(),
                move |app: &AppHandle<R>, _shortcut: &Shortcut, _event: ShortcutEvent| {
                    let _ = app.emit("hotkey-pressed", act);
                },
            )
            .map_err(|e| format!("Failed to register hotkey {hk_str}: {e}"))?;
    }

    Ok(())
}


