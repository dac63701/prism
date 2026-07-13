use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

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
    #[allow(dead_code)]
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

        let base = server_url.trim_end_matches('/');
        let session = Uuid::new_v4().to_string();

        let auth_url = format!(
            "{}/api/auth/google?desktop=true&session={}&next=/dashboard",
            base, session
        );
        let _ = tauri_plugin_opener::open_url(&auth_url, None::<&str>);

        let poll_url = format!("{}/api/auth/desktop/poll?session={}", base, session);
        let app_handle = app.clone();

        tauri::async_runtime::spawn(async move {
            let client = reqwest::Client::new();
            let start = std::time::Instant::now();

            loop {
                if start.elapsed().as_secs() > 300 {
                    let _ = app_handle.emit("auth-error", "Sign in timed out — please try again");
                    break;
                }

                {
                    let mgr = app_handle.state::<AuthManager>();
                    let authed = mgr.state.lock().map(|s| s.authenticated).unwrap_or(false);
                    if authed {
                        break;
                    }
                }

                match client.get(&poll_url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(body) = resp.json::<Value>().await {
                            if let Some(code) = body.get("code").and_then(|c| c.as_str()) {
                                let _ = AuthManager::handle_callback(&app_handle, code.to_string())
                                    .await;
                                break;
                            }
                        }
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        eprintln!("[auth] poll returned {} {} — will retry", status, body);
                    }
                    Err(e) => {
                        eprintln!("[auth] poll request failed: {e}");
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });
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

        // Attempt API key creation (non-fatal — primary auth is now JWT)
        match Self::create_api_key(&client, base, &exchange.access_token).await {
            Ok(key) => {
                let settings_mgr = app.state::<SettingsManager>();
                let mut s = settings_mgr.get();
                s.cloud.api_key = key;
                let _ = settings_mgr.set(app, s);
            }
            Err(e) => {
                eprintln!("[auth] API key creation failed (non-fatal): {e}");
            }
        }

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
            new_settings.cloud.access_token = exchange.access_token;
            new_settings.cloud.refresh_token = exchange.refresh_token;
            new_settings.cloud.account_display_name = exchange.user.display_name;
            new_settings.cloud.account_email = exchange.user.email;
            let _ = settings_mgr.set(app, new_settings);
        }

        let _ = app.emit("auth-state-changed", true);
        Ok(())
    }

    /// Refresh the access_token using the stored refresh_token.
    /// Returns the new access_token and updates settings.
    pub async fn refresh_access_token(app: &AppHandle) -> Result<String, String> {
        let settings = app.state::<SettingsManager>().get();
        let base = settings.cloud.server_url.trim_end_matches('/').to_string();
        let refresh_token = settings.cloud.refresh_token;
        if refresh_token.is_empty() {
            return Err("No refresh token available".into());
        }

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{base}/api/auth/refresh"))
            .json(&serde_json::json!({ "refresh_token": refresh_token }))
            .send()
            .await
            .map_err(|e| format!("Refresh request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Token refresh failed ({status}): {body}"));
        }

        #[derive(Deserialize)]
        struct RefreshResponse {
            access_token: String,
            #[allow(dead_code)]
            refresh_token: Option<String>,
        }

        let body: RefreshResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse refresh response: {e}"))?;

        let new_token = body.access_token;

        {
            let settings_mgr = app.state::<SettingsManager>();
            let mut s = settings_mgr.get();
            s.cloud.access_token = new_token.clone();
            if let Some(new_refresh) = body.refresh_token {
                s.cloud.refresh_token = new_refresh;
            }
            let _ = settings_mgr.set(app, s);
        }

        Ok(new_token)
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
        new_settings.cloud.access_token.clear();
        new_settings.cloud.refresh_token.clear();
        new_settings.cloud.account_display_name.clear();
        new_settings.cloud.account_email.clear();
        let _ = settings_mgr.set(app, new_settings);

        {
            let mgr = app.state::<AuthManager>();
            if let Ok(mut state) = mgr.state.lock() {
                state.authenticated = false;
                state.display_name.clear();
                state.email.clear();
            };
        }

        let _ = app.emit("auth-state-changed", false);
    }
}
