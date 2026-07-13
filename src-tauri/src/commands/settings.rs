//! Settings IPC commands — get, update, reset.

use parking_lot::Mutex;
use tauri::State;

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
/// Recording is never started or stopped here. The clip duration applies
/// immediately; other capture settings apply before the next session.
#[tauri::command]
pub async fn update_settings(
    app: tauri::AppHandle,
    manager: State<'_, SettingsManager>,
    recorder: State<'_, Mutex<Recorder>>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let updated = manager.set(&app, settings).map_err(|e| e.to_string())?;
    let recorder = recorder.lock();
    if recorder.is_recording() {
        recorder.set_buffer_duration_secs(updated.recording.buffer_duration_secs);
    } else {
        recorder.reconfigure(&updated);
    }
    hotkey::register_hotkeys(&app, &updated.hotkeys)?;
    Ok(updated)
}

/// Reset settings to factory defaults.
#[tauri::command]
pub async fn reset_settings(
    app: tauri::AppHandle,
    manager: State<'_, SettingsManager>,
    recorder: State<'_, Mutex<Recorder>>,
) -> Result<AppSettings, String> {
    let updated = manager.reset(&app).map_err(|e| e.to_string())?;
    let recorder = recorder.lock();
    if recorder.is_recording() {
        recorder.set_buffer_duration_secs(updated.recording.buffer_duration_secs);
    } else {
        recorder.reconfigure(&updated);
    }
    hotkey::register_hotkeys(&app, &updated.hotkeys)?;
    Ok(updated)
}

/// Validate a hotkey string before saving it.
#[tauri::command]
pub async fn validate_hotkey(hotkey_str: String) -> Result<(), String> {
    hotkey::parse_hotkey(&hotkey_str).map(|_| ())
}
