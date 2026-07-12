use tauri::{AppHandle, State};

use crate::auth::AuthManager;
use crate::settings::SettingsManager;

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

/// Verify the stored API key is still valid against the server.
/// Returns true if the key is valid or the server is unreachable (to
/// avoid clearing auth state due to transient network issues).
/// Only returns false on a definitive 401 rejection from the server.
#[tauri::command]
pub async fn cloud_verify_auth(settings_mgr: State<'_, SettingsManager>) -> Result<bool, String> {
    let settings = settings_mgr.get();
    let api_key = settings.cloud.api_key;
    if api_key.is_empty() || !api_key.starts_with("prism_") {
        return Ok(false);
    }
    let base_url = settings.cloud.server_url.trim_end_matches('/').to_string();
    if base_url.is_empty() {
        return Ok(false);
    }
    let upload_url = format!("{base_url}/api/clips/upload");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let req = client
        .post(&upload_url)
        .header("Authorization", format!("Bearer {api_key}"));

    match req.send().await {
        Ok(resp) => {
            let valid = resp.status() != reqwest::StatusCode::UNAUTHORIZED;
            if !valid {
                eprintln!("[auth] API key rejected by {} (401)", upload_url);
            }
            Ok(valid)
        }
        Err(e) => {
            // Network / timeout / DNS error — keep current auth state
            eprintln!("[auth] verify request failed (will not invalidate): {e}");
            Ok(true)
        }
    }
}
