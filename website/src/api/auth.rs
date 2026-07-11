use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, Redirect},
    Json,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{jwt, AuthUser};
use crate::config::Config;
use crate::db;
use crate::errors::AppError;
use crate::storage::StorageBackend;

#[derive(Deserialize)]
pub struct RegisterRequest {
    email: String,
    password: String,
    display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    display_name: String,
}

#[derive(Deserialize)]
pub struct CreateApiKeyRequest {
    name: Option<String>,
}

#[derive(Deserialize)]
pub struct GoogleStartQuery {
    next: Option<String>,
    desktop: Option<bool>,
}

#[derive(Deserialize)]
pub struct GoogleCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct DesktopExchangeRequest {
    code: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    user: UserResponse,
    access_token: String,
    refresh_token: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    id: Uuid,
    email: String,
    display_name: String,
    avatar_url: Option<String>,
    google_connected: bool,
    role: String,
    storage_used_bytes: i64,
    max_storage_bytes: i64,
    created_at: String,
}

#[derive(Serialize)]
pub struct ApiKeyCreatedResponse {
    key: String,
    key_id: Uuid,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    expires_in: Option<i64>,
    #[allow(dead_code)]
    refresh_token: Option<String>,
    #[allow(dead_code)]
    scope: Option<String>,
    #[allow(dead_code)]
    token_type: Option<String>,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: String,
    name: Option<String>,
    picture: Option<String>,
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Password hash error: {e}")))?
        .to_string())
}

fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed = match PasswordHash::new(hash) {
        Ok(hash) => hash,
        Err(_) => return Ok(false),
    };
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

fn generate_api_key() -> (String, String, String) {
    let key_bytes: [u8; 32] = rand::thread_rng().gen();
    let key_hex = hex::encode(key_bytes);
    let full_key = format!("prism_{}", key_hex);
    let prefix: String = key_hex.chars().take(12).collect();

    let mut hasher = Sha256::new();
    hasher.update(full_key.as_bytes());
    let hash = hex::encode(hasher.finalize());

    (full_key, hash, prefix)
}

fn user_to_response(user: &db::users::User) -> UserResponse {
    UserResponse {
        id: user.id,
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        avatar_url: user.avatar_url.clone(),
        google_connected: user.google_id.is_some(),
        role: user.role.clone(),
        storage_used_bytes: user.storage_used_bytes,
        max_storage_bytes: user.max_storage_bytes,
        created_at: user.created_at.to_rfc3339(),
    }
}

fn make_auth_response(user: &db::users::User, config: &Config) -> Result<AuthResponse, AppError> {
    let access_token = jwt::create_access_token(user.id, &user.role, &config.jwt_secret)?;
    let refresh_token = jwt::create_refresh_token(user.id, &config.jwt_secret)?;

    Ok(AuthResponse {
        user: user_to_response(user),
        access_token,
        refresh_token,
    })
}

fn cookie_secure(config: &Config) -> bool {
    config.public_url().starts_with("https://")
}

fn auth_cookie_headers(access: &str, refresh: &str, config: &Config) -> HeaderMap {
    let secure = cookie_secure(config);
    let mut headers = HeaderMap::new();

    let access_cookie = format!(
        "prism_session={access}; Path=/; HttpOnly; SameSite=Lax; Max-Age=900{}",
        if secure { "; Secure" } else { "" }
    );
    let refresh_cookie = format!(
        "prism_refresh={refresh}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000{}",
        if secure { "; Secure" } else { "" }
    );

    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&access_cookie).expect("valid access cookie"),
    );
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&refresh_cookie).expect("valid refresh cookie"),
    );
    headers
}

fn clear_cookie_headers(config: &Config) -> HeaderMap {
    let secure = cookie_secure(config);
    let mut headers = HeaderMap::new();
    for name in ["prism_session", "prism_refresh"] {
        let cookie = format!(
            "{name}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
            if secure { "; Secure" } else { "" }
        );
        headers.append(
            header::SET_COOKIE,
            HeaderValue::from_str(&cookie).expect("valid cookie"),
        );
    }
    headers
}

fn extract_cookie_from_headers(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for pair in cookie_header.split(';') {
        let mut pieces = pair.trim().splitn(2, '=');
        let key = pieces.next()?.trim();
        let value = pieces.next()?.trim();
        if key == cookie_name {
            return Some(value.to_string());
        }
    }
    None
}

async fn issue_user_auth(user: &db::users::User, config: &Config) -> Result<(HeaderMap, Json<AuthResponse>), AppError> {
    let response = make_auth_response(user, config)?;
    Ok((
        auth_cookie_headers(&response.access_token, &response.refresh_token, config),
        Json(response),
    ))
}

fn default_display_name(email: &str, fallback: &str) -> String {
    email
        .split('@')
        .next()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(fallback)
        .to_string()
}

async fn sync_google_user(
    pool: &PgPool,
    config: &Config,
    google: &GoogleUserInfo,
) -> Result<db::users::User, AppError> {
    let max_bytes = (config.default_max_storage_gb * 1_073_741_824) as i64;
    let display_name = google
        .name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default_display_name(&google.email, "Prism User"));

    if let Some(existing) = db::users::get_user_by_google_id(pool, &google.sub).await? {
        if existing.avatar_url.as_deref() != google.picture.as_deref() {
            db::users::update_user_avatar(pool, existing.id, google.picture.as_deref()).await?;
        }
        if existing.display_name != display_name {
            db::users::update_user_profile(pool, existing.id, &display_name).await?;
        }
        return Ok(db::users::get_user_by_id(pool, existing.id)
            .await?
            .ok_or(AppError::Unauthorized)?);
    }

    if let Some(existing_email_user) = db::users::get_user_by_email(pool, &google.email).await? {
        db::users::link_google_account(
            pool,
            existing_email_user.id,
            &google.sub,
            google.picture.as_deref(),
        )
        .await?;
        if existing_email_user.display_name != display_name {
            db::users::update_user_profile(pool, existing_email_user.id, &display_name).await?;
        }
        return Ok(db::users::get_user_by_id(pool, existing_email_user.id)
            .await?
            .ok_or(AppError::Unauthorized)?);
    }

    let user = db::users::create_google_user(
        pool,
        &google.email,
        &display_name,
        max_bytes,
        &google.sub,
        google.picture.as_deref(),
    )
    .await?;

    Ok(user)
}

pub async fn google_start(
    State(config): State<Config>,
    Query(query): Query<GoogleStartQuery>,
) -> Result<Redirect, AppError> {
    if config.google_client_id.is_empty()
        || config.google_client_secret.is_empty()
        || config.google_redirect_uri.is_empty()
    {
        return Err(AppError::BadRequest(
            "Google OAuth is not configured".into(),
        ));
    }

    let redirect_to = query.next.unwrap_or_else(|| "/dashboard".into());
    let desktop = query.desktop.unwrap_or(false);
    let state = jwt::create_oauth_state(&redirect_to, desktop, &config.jwt_secret)?;

    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent%20select_account&state={}",
        urlencoding::encode(&config.google_client_id),
        urlencoding::encode(&config.google_redirect_uri),
        urlencoding::encode("openid email profile"),
        urlencoding::encode(&state)
    );

    Ok(Redirect::temporary(&url))
}

pub async fn google_callback(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Query(query): Query<GoogleCallbackQuery>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if let Some(error) = query.error {
        return Err(AppError::BadRequest(format!("Google login failed: {error}")));
    }

    let code = query
        .code
        .ok_or_else(|| AppError::BadRequest("Missing Google code".into()))?;
    let state = query
        .state
        .ok_or_else(|| AppError::BadRequest("Missing state".into()))?;

    let claims = jwt::verify_oauth_state(&state, &config.jwt_secret)?;

    let client = reqwest::Client::new();
    let token = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code.as_str()),
            ("client_id", config.google_client_id.as_str()),
            ("client_secret", config.google_client_secret.as_str()),
            ("redirect_uri", config.google_redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Google token request failed: {e}")))?
        .error_for_status()
        .map_err(|e| AppError::Internal(format!("Google token response error: {e}")))?
        .json::<TokenResponse>()
        .await
        .map_err(|e| AppError::Internal(format!("Google token decode failed: {e}")))?;

    let google_user = client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(&token.access_token)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Google userinfo request failed: {e}")))?
        .error_for_status()
        .map_err(|e| AppError::Internal(format!("Google userinfo response error: {e}")))?
        .json::<GoogleUserInfo>()
        .await
        .map_err(|e| AppError::Internal(format!("Google userinfo decode failed: {e}")))?;

    let user = sync_google_user(&pool, &config, &google_user).await?;
    let auth = make_auth_response(&user, &config)?;
    let cookies = auth_cookie_headers(&auth.access_token, &auth.refresh_token, &config);

    if claims.desktop {
        let desktop_code = jwt::create_desktop_code(user.id, &user.role, &config.jwt_secret)?;
        let target = format!("/api/auth/desktop/success?code={}", urlencoding::encode(&desktop_code));
        return Ok((cookies, Redirect::temporary(&target)));
    }

    let redirect_to = if claims.redirect_to.is_empty() {
        "/dashboard".to_string()
    } else {
        claims.redirect_to
    };

    Ok((cookies, Redirect::temporary(&redirect_to)))
}

#[derive(Deserialize)]
pub struct DesktopSuccessQuery {
    code: String,
}

/// Renders a branded "Signed in" page that prompts the user to open the
/// desktop app via the `prism://` custom scheme.
pub async fn desktop_success(
    Query(query): Query<DesktopSuccessQuery>,
    State(config): State<Config>,
) -> Html<String> {
    let app_url = format!("{}?code={}", config.desktop_scheme_url, urlencoding::encode(&query.code));

    Html(format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Prism — Signed In</title>
<meta http-equiv="refresh" content="2;url={app_url}">
<style>
  * {{ margin:0; padding:0; box-sizing:border-box; }}
  body {{
    font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Oxygen,Ubuntu,sans-serif;
    display:flex; justify-content:center; align-items:center;
    min-height:100vh; background:#09090b; color:#e4e4e7;
  }}
  .card {{ text-align:center; padding:48px 24px; max-width:420px; }}
  .logo {{ width:64px; height:64px; margin:0 auto 24px; }}
  h1 {{ font-size:24px; font-weight:600; margin-bottom:8px; }}
  p {{ color:#a1a1aa; margin-bottom:28px; line-height:1.5; }}
  .btn {{
    display:inline-block; background:#6366f1; color:#fff;
    border:none; padding:12px 32px; border-radius:8px;
    font-size:15px; font-weight:500; cursor:pointer;
    text-decoration:none; transition:background .15s;
  }}
  .btn:hover {{ background:#4f46e5; }}
  .fallback {{ margin-top:20px; font-size:13px; color:#71717a; }}
  .fallback a {{ color:#818cf8; }}
</style>
</head>
<body>
<div class="card">
  <svg class="logo" viewBox="0 0 64 64" fill="none">
    <rect width="64" height="64" rx="16" fill="#6366f1"/>
    <path d="M20 44V20h8l8 12 8-12h8v24h-8V32l-8 12-8-12v12H20z" fill="#fff"/>
  </svg>
  <h1>Signed in to Prism</h1>
  <p>Your Google account is connected.<br>Return to the app to continue.</p>
  <a class="btn" href="{app_url}">Open Prism App</a>
  <p class="fallback">
    Not opening? <a href="{app_url}">Click here</a> or go back to the app and sign in again.
  </p>
</div>
</body>
</html>"##))
}

pub async fn desktop_exchange(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(body): Json<DesktopExchangeRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    let claims = jwt::verify_desktop_code(&body.code, &config.jwt_secret)?;
    let user = db::users::get_user_by_id(&pool, claims.sub)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let auth = make_auth_response(&user, &config)?;
    Ok((StatusCode::OK, Json(auth)))
}

pub async fn register(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(body): Json<RegisterRequest>,
) -> Result<(HeaderMap, Json<AuthResponse>), AppError> {
    if body.email.is_empty() || !body.email.contains('@') {
        return Err(AppError::BadRequest("Invalid email address".into()));
    }
    if body.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters".into(),
        ));
    }

    let existing = db::users::get_user_by_email(&pool, &body.email).await?;
    if existing.is_some() {
        return Err(AppError::Conflict("Email already registered".into()));
    }

    let display_name = body
        .display_name
        .unwrap_or_else(|| default_display_name(&body.email, "Prism User"));
    let password_hash = hash_password(&body.password)?;
    let max_bytes = (config.default_max_storage_gb * 1_073_741_824) as i64;

    let user = db::users::create_user(
        &pool,
        &body.email,
        &password_hash,
        &display_name,
        max_bytes,
        None,
        None,
    )
    .await?;

    tracing::info!(user_id = %user.id, "user_registered");
    Ok(issue_user_auth(&user, &config).await?)
}

pub async fn login(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(body): Json<LoginRequest>,
) -> Result<(HeaderMap, Json<AuthResponse>), AppError> {
    let user = match db::users::get_user_by_email(&pool, &body.email).await {
        Ok(Some(u)) => u,
        Ok(None) => return Err(AppError::Unauthorized),
        Err(e) => {
            tracing::error!(error = %e, "login_db_error");
            return Err(AppError::Internal("Database error during login".into()));
        }
    };

    if user.is_banned {
        return Err(AppError::Forbidden);
    }

    match verify_password(&body.password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => return Err(AppError::Unauthorized),
        Err(e) => {
            tracing::error!(error = %e, "login_password_verify_error");
            return Err(AppError::Internal("Password verification failed".into()));
        }
    }

    match issue_user_auth(&user, &config).await {
        Ok(response) => {
            tracing::info!(user_id = %user.id, "user_logged_in");
            Ok(response)
        }
        Err(e) => {
            tracing::error!(user_id = %user.id, error = %e, "login_token_issuance_error");
            Err(AppError::Internal("Failed to issue authentication tokens".into()))
        }
    }
}

pub async fn refresh(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    headers: HeaderMap,
    Json(body): Json<RefreshRequest>,
) -> Result<(HeaderMap, Json<AuthResponse>), AppError> {
    let refresh_token = body
        .refresh_token
        .or_else(|| extract_cookie_from_headers(&headers, "prism_refresh"))
        .ok_or(AppError::Unauthorized)?;

    let claims = jwt::verify_refresh_token(&refresh_token, &config.jwt_secret)?;

    let user = db::users::get_user_by_id(&pool, claims.sub)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if user.is_banned {
        return Err(AppError::Forbidden);
    }

    Ok(issue_user_auth(&user, &config).await?)
}

pub async fn logout(State(config): State<Config>) -> Result<(HeaderMap, Json<serde_json::Value>), AppError> {
    Ok((
        clear_cookie_headers(&config),
        Json(serde_json::json!({"status": "ok"})),
    ))
}

pub async fn me(State(pool): State<PgPool>, auth: AuthUser) -> Result<Json<UserResponse>, AppError> {
    let user = db::users::get_user_by_id(&pool, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(user_to_response(&user)))
}

pub async fn change_password(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = db::users::get_user_by_id(&pool, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !verify_password(&body.current_password, &user.password_hash)? {
        return Err(AppError::BadRequest("Current password is incorrect".into()));
    }

    if body.new_password.len() < 8 {
        return Err(AppError::BadRequest(
            "New password must be at least 8 characters".into(),
        ));
    }

    let new_hash = hash_password(&body.new_password)?;
    db::users::update_user_password(&pool, auth.user_id, &new_hash).await?;

    Ok(Json(serde_json::json!({"status": "ok"})))
}

pub async fn update_profile(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<UserResponse>, AppError> {
    db::users::update_user_profile(&pool, auth.user_id, &body.display_name).await?;

    let user = db::users::get_user_by_id(&pool, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(user_to_response(&user)))
}

pub async fn delete_account(
    State(pool): State<PgPool>,
    State(storage): State<crate::storage::local::LocalStorage>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = db::users::get_user_by_id(&pool, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let (clips, _) = db::clips::list_clips(&pool, Some(user.id), "", "", "", "created_at", "desc", 1, 1000)
        .await?;
    for clip in clips {
        if let Some(stored) = db::clips::get_clip(&pool, clip.id).await? {
            let _ = storage.delete(&stored.storage_path).await;
            if let Some(thumb) = &stored.thumbnail_path {
                let _ = storage.delete(thumb).await;
            }
            let _ = db::clips::delete_clip(&pool, stored.id).await;
        }
    }

    db::users::delete_user(&pool, user.id).await?;
    tracing::info!(user_id = %user.id, "user_deleted");

    Ok(Json(serde_json::json!({"status": "ok"})))
}

pub async fn list_api_keys(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> Result<Json<Vec<db::api_keys::ApiKeyListItem>>, AppError> {
    let keys = db::api_keys::list_api_keys(&pool, auth.user_id).await?;
    Ok(Json(keys))
}

pub async fn create_api_key(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(body): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyCreatedResponse>, AppError> {
    let (full_key, hash, prefix) = generate_api_key();
    let name = body.name.unwrap_or_default();

    let key = db::api_keys::insert_api_key(&pool, auth.user_id, &name, &hash, &prefix).await?;

    tracing::info!(user_id = %auth.user_id, key_id = %key.id, "api_key_created");

    Ok(Json(ApiKeyCreatedResponse {
        key: full_key,
        key_id: key.id,
    }))
}

pub async fn revoke_api_key(
    State(pool): State<PgPool>,
    auth: AuthUser,
    axum::extract::Path(key_id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let revoked = db::api_keys::revoke_api_key(&pool, key_id, auth.user_id).await?;
    if !revoked {
        return Err(AppError::NotFound("API key not found".into()));
    }

    tracing::info!(user_id = %auth.user_id, key_id = %key_id, "api_key_revoked");

    Ok(Json(serde_json::json!({"status": "ok"})))
}

pub async fn log_activity(
    pool: &PgPool,
    user_id: Uuid,
    action: &str,
    details: Option<&serde_json::Value>,
) {
    let result = sqlx::query(
        r#"INSERT INTO activity_logs (user_id, action, details)
           VALUES ($1, $2::log_action, $3::jsonb)"#,
    )
    .bind(user_id)
    .bind(action)
    .bind(details.map(|v| v.to_string()))
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::warn!(%user_id, %action, error = %e, "Failed to log activity");
    }
}
