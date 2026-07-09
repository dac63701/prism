//! Settings IPC commands — get, update, reset.

use std::sync::Mutex;

use tauri::{Emitter, State};

use crate::hotkey;
use crate::recording::Recorder;
use crate::settings::config::AppSettings;
use crate::settings::SettingsManager;

/// Get the current app settings.
#[tauri::command]
pub async fn get_settings(manager: State<'_, SettingsManager>) -> Result<AppSettings, String> {
    Ok(manager.get())
}

/// Update settings with new values.
/// Returns the updated settings on success.
#[tauri::command]
pub async fn update_settings(
    app: tauri::AppHandle,
    manager: State<'_, SettingsManager>,
    settings: AppSettings,
    recorder: State<'_, Mutex<Recorder>>,
) -> Result<AppSettings, String> {
    let updated = manager.set(&app, settings).map_err(|e| e.to_string())?;
    hotkey::register_hotkeys(&app, &updated.hotkeys)?;

    let rec = recorder.lock().map_err(|e| e.to_string())?;
    let was_recording = rec.is_recording();
    if was_recording {
        rec.stop_recording().ok();
    }
    rec.reconfigure(&updated);
    if was_recording {
        rec.start_recording()?;
        rec.start_polling(app.clone());
        let _ = app.emit("recording-state-changed", true);
    }

    Ok(updated)
}

/// Reset settings to factory defaults.
#[tauri::command]
pub async fn reset_settings(
    app: tauri::AppHandle,
    manager: State<'_, SettingsManager>,
) -> Result<AppSettings, String> {
    let updated = manager.reset(&app).map_err(|e| e.to_string())?;
    hotkey::register_hotkeys(&app, &updated.hotkeys)?;
    Ok(updated)
}

/// Validate a hotkey string before saving it.
#[tauri::command]
pub async fn validate_hotkey(hotkey_str: String) -> Result<(), String> {
    hotkey::parse_hotkey(&hotkey_str).map(|_| ())
}
