use tauri::{AppHandle, State};

use crate::auth::AuthManager;

#[tauri::command]
pub async fn cloud_login(app: AppHandle) -> Result<(), String> {
    AuthManager::start_login(&app);
    Ok(())
}

#[tauri::command]
pub async fn cloud_logout(app: AppHandle) -> Result<(), String> {
    AuthManager::logout(&app);
    Ok(())
}

#[tauri::command]
pub async fn get_auth_status(
    auth_mgr: State<'_, AuthManager>,
) -> Result<crate::auth::AuthState, String> {
    let state = auth_mgr.state.lock().map_err(|e| e.to_string())?;
    Ok(state.clone())
}

#[tauri::command]
pub async fn cloud_handle_auth_code(app: AppHandle, code: String) -> Result<(), String> {
    AuthManager::handle_callback(&app, code).await
}

/// Check whether the stored API key has valid format.
/// Does NOT contact the server — the runtime upload path handles
/// server-side validation and surfaces 401 errors naturally.
#[tauri::command]
pub async fn cloud_verify_auth(
    settings_mgr: State<'_, crate::settings::SettingsManager>,
) -> Result<bool, String> {
    let api_key = settings_mgr.get().cloud.api_key;
    Ok(!api_key.is_empty() && api_key.starts_with("prism_"))
}
