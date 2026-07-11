use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::settings::SettingsManager;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthState {
    pub authenticated: bool,
    pub display_name: String,
    pub email: String,
}

#[derive(Default)]
pub struct AuthManager {
    pub state: Mutex<AuthState>,
}

#[derive(Deserialize)]
struct DesktopExchangeResponse {
    user: DesktopUser,
    access_token: String,
    refresh_token: String,
}

#[derive(Deserialize)]
struct DesktopUser {
    id: String,
    email: String,
    display_name: String,
    #[allow(dead_code)]
    avatar_url: Option<String>,
    #[allow(dead_code)]
    google_connected: bool,
    #[allow(dead_code)]
    role: String,
    #[allow(dead_code)]
    storage_used_bytes: i64,
    #[allow(dead_code)]
    max_storage_bytes: i64,
    #[allow(dead_code)]
    created_at: String,
}

#[derive(Deserialize)]
struct ApiKeyCreated {
    key: String,
    #[allow(dead_code)]
    key_id: String,
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(AuthState::default()),
        }
    }

    pub fn start_login(app: &AppHandle) {
        let settings = app.state::<SettingsManager>().get();
        let server_url = settings.cloud.server_url;
        if server_url.is_empty() {
            let _ = app.emit("auth-error", "Server URL not configured");
            return;
        }
        let auth_url = format!(
            "{}/api/auth/google?desktop=true&next=/dashboard",
            server_url.trim_end_matches('/')
        );
        let _ = tauri_plugin_opener::open_url(&auth_url, None::<&str>);
    }

    pub async fn handle_callback(app: &AppHandle, code: String) -> Result<(), String> {
        let settings = app.state::<SettingsManager>().get();
        let server_url = settings.cloud.server_url;
        if server_url.is_empty() {
            return Err("Server URL not configured".into());
        }

        let base = server_url.trim_end_matches('/');

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{base}/api/auth/desktop/exchange"))
            .json(&serde_json::json!({ "code": code }))
            .send()
            .await
            .map_err(|e| format!("Exchange request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Exchange failed ({status}): {body}"));
        }

        let exchange: DesktopExchangeResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse exchange response: {e}"))?;

        let api_key = Self::create_api_key(&client, base, &exchange.access_token).await?;

        {
            let mgr = app.state::<AuthManager>();
            let mut state = mgr.state.lock().map_err(|e| e.to_string())?;
            state.authenticated = true;
            state.display_name = exchange.user.display_name.clone();
            state.email = exchange.user.email.clone();
        }

        {
            let settings_mgr = app.state::<SettingsManager>();
            let mut new_settings = settings_mgr.get();
            new_settings.cloud.api_key = api_key;
            new_settings.cloud.account_display_name = exchange.user.display_name;
            new_settings.cloud.account_email = exchange.user.email;
            let _ = settings_mgr.set(app, new_settings);
        }

        let _ = app.emit("auth-state-changed", true);
        Ok(())
    }

    async fn create_api_key(
        client: &reqwest::Client,
        base: &str,
        access_token: &str,
    ) -> Result<String, String> {
        let resp = client
            .post(format!("{base}/api/auth/api-keys"))
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&serde_json::json!({ "name": "Prism Desktop App" }))
            .send()
            .await
            .map_err(|e| format!("API key request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API key creation failed ({status}): {body}"));
        }

        let key: ApiKeyCreated = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse API key response: {e}"))?;

        Ok(key.key)
    }

    pub fn logout(app: &AppHandle) {
        let settings_mgr = app.state::<SettingsManager>();
        let mut new_settings = settings_mgr.get();
        new_settings.cloud.api_key.clear();
        new_settings.cloud.account_display_name.clear();
        new_settings.cloud.account_email.clear();
        let _ = settings_mgr.set(app, new_settings);

        {
            let mgr = app.state::<AuthManager>();
            let mut state = mgr.state.lock().map_err(|e| e.to_string()).unwrap();
            state.authenticated = false;
            state.display_name.clear();
            state.email.clear();
        }

        let _ = app.emit("auth-state-changed", false);
    }
}
