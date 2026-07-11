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
