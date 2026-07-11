//! Settings IPC commands — get, update, reset.

use tauri::State;

use crate::hotkey;
use crate::settings::config::AppSettings;
use crate::settings::SettingsManager;

/// Get the current app settings.
#[tauri::command]
pub async fn get_settings(manager: State<'_, SettingsManager>) -> Result<AppSettings, String> {
    Ok(manager.get())
}

/// Update settings with new values.
/// Returns the updated settings on success.
/// Recording state is never touched here — only the on/off button
/// controls start/stop. New capture settings (resolution, FPS, etc.)
/// take effect on the next recording session or app restart.
#[tauri::command]
pub async fn update_settings(
    app: tauri::AppHandle,
    manager: State<'_, SettingsManager>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let updated = manager.set(&app, settings).map_err(|e| e.to_string())?;
    hotkey::register_hotkeys(&app, &updated.hotkeys)?;
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
