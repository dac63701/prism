use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use regex::Regex;
use std::sync::OnceLock;

use crate::auth::{jwt, AuthUser};
use crate::config::Config;
use crate::db;
use crate::email;
use crate::errors::AppError;
use crate::storage::StorageBackend;

pub struct CachedCode {
    pub code: String,
    pub created_at: DateTime<Utc>,
}

pub type DesktopCodeCache = Arc<Mutex<HashMap<String, CachedCode>>>;

pub fn new_desktop_code_cache() -> DesktopCodeCache {
    Arc::new(Mutex::new(HashMap::new()))
}

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
    session: Option<String>,
}

#[derive(Deserialize)]
pub struct DesktopPollQuery {
    session: String,
}

#[derive(Serialize)]
pub struct PollResponse {
    code: String,
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

#[derive(Deserialize)]
pub struct VerifyEmailQuery {
    token: String,
}

#[derive(Deserialize)]
pub struct ResendVerificationRequest {
    email: String,
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
    email_verified: bool,
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
        Err(e) => {
            tracing::warn!("Failed to parse password hash: {e}");
            return Ok(false);
        },
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
        email_verified: user.email_verified_at.is_some(),
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

/// Validate email format using a regex.
fn is_valid_email(email: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(
            r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,253}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,253}[a-zA-Z0-9])?)*\.[a-zA-Z]{2,}$"
        ).expect("valid email regex")
    });
    re.is_match(email)
}

fn generate_verification_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
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
        // Google-authenticated users are verified
        if existing_email_user.email_verified_at.is_none() {
            db::users::verify_email(pool, existing_email_user.id).await?;
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
    let session = query.session.clone();
    let state = jwt::create_oauth_state(&redirect_to, desktop, session, &config.jwt_secret)?;

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
    State(cache): State<DesktopCodeCache>,
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

        if let Some(session) = &claims.session {
            if let Ok(mut map) = cache.lock() {
                map.insert(session.clone(), CachedCode {
                    code: desktop_code.clone(),
                    created_at: Utc::now(),
                });
            }
        }

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
    let site_url = config.site_url.trim_end_matches('/');

    Html(format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Prism — Signed In</title>
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
  .redirecting {{ margin-top:20px; font-size:13px; color:#52525b; }}
</style>
</head>
<body>
<div class="card">
  <img class="logo" src="/brand/logo.svg" alt="Prism">
  <h1>Signed in to Prism</h1>
  <p>Your Google account is connected.<br>Return to the app to continue.</p>
  <a class="btn" href="{app_url}">Open Prism App</a>
  <p class="redirecting" id="redirectMsg">Redirecting to Prism&hellip;</p>
</div>
<script>
  var deepLink = "{app_url}";
  var successUrl = "{site_url}/signin/success";

  // Attempt to open the deep link via a programmatic anchor click.
  // This works in more browsers than window.location.href for custom
  // protocol schemes (prism://), which many browsers block from
  // script-initiated top-level navigation.
  var opened = false;
  function tryDeepLink() {{
    var a = document.createElement("a");
    a.setAttribute("href", deepLink);
    a.style.display = "none";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
  }}

  window.addEventListener("blur", function() {{
    // Browser navigated away (deep link likely worked)
    opened = true;
  }});

  // If deep link didn't navigate away, redirect to success page
  setTimeout(function() {{
    if (!opened) {{
      window.location.replace(successUrl);
    }}
  }}, 3000);

  // On button click, just let the native anchor href fire (don't
  // preventDefault) — that way the browser handles the custom protocol
  // navigation directly, which is the most reliable path.
  // After a short delay, redirect to the success page.
  var btn = document.querySelector(".btn");
  if (btn) {{
    btn.addEventListener("click", function() {{
      setTimeout(function() {{
        window.location.replace(successUrl);
      }}, 1000);
    }});
  }}

  // Also try immediately (auto-open)
  tryDeepLink();
</script>
</body>
</html>"##))
}

pub async fn desktop_poll(
    Query(query): Query<DesktopPollQuery>,
    State(cache): State<DesktopCodeCache>,
) -> Result<Json<PollResponse>, AppError> {
    if let Ok(mut map) = cache.lock() {
        if let Some(entry) = map.remove(&query.session) {
            if Utc::now().signed_duration_since(entry.created_at).num_seconds() < 300 {
                return Ok(Json(PollResponse { code: entry.code }));
            }
        }
    }
    Err(AppError::NotFound("No auth code available for this session".into()))
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
) -> Result<Json<serde_json::Value>, AppError> {
    let email = body.email.trim().to_lowercase();

    if email.is_empty() || !is_valid_email(&email) {
        return Err(AppError::BadRequest("Invalid email address".into()));
    }
    if body.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters".into(),
        ));
    }

    let existing = db::users::get_user_by_email(&pool, &email).await?;
    if existing.is_some() {
        return Err(AppError::Conflict("Email already registered".into()));
    }

    let display_name = body
        .display_name
        .unwrap_or_else(|| default_display_name(&email, "Prism User"));
    let password_hash = hash_password(&body.password)?;
    let max_bytes = (config.default_max_storage_gb * 1_073_741_824) as i64;
    let verification_token = generate_verification_token();

    let user = db::users::create_user(
        &pool,
        &email,
        &password_hash,
        &display_name,
        max_bytes,
        None,
        None,
        Some(&verification_token),
    )
    .await?;

    // Send verification email (non-blocking — log and continue on failure)
    let send_result = email::send_verification_email(
        &config,
        &user.email,
        &user.display_name,
        &verification_token,
    )
    .await;

    match send_result {
        Ok(()) => tracing::info!(user_id = %user.id, "verification email sent"),
        Err(e) => tracing::warn!(user_id = %user.id, error = %e, "failed to send verification email"),
    }

    tracing::info!(user_id = %user.id, "user_registered");

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Account created. Please check your email to verify your account.",
        "email": user.email,
    })))
}

pub async fn verify_email(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Query(query): Query<VerifyEmailQuery>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if query.token.is_empty() {
        return Err(AppError::BadRequest("Missing verification token".into()));
    }

    let user = db::users::get_user_by_verification_token(&pool, &query.token)
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired verification token".into()))?;

    if user.email_verified_at.is_some() {
        // Already verified — redirect to dashboard
        return Ok(Redirect::temporary(&format!("{}/dashboard", config.site_url.trim_end_matches('/'))));
    }

    db::users::verify_email(&pool, user.id).await?;

    tracing::info!(user_id = %user.id, "email_verified");

    let redirect = format!("{}/login?verified=1", config.site_url.trim_end_matches('/'));
    Ok(Redirect::temporary(&redirect))
}

pub async fn resend_verification(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(body): Json<ResendVerificationRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let email = body.email.trim().to_lowercase();

    if email.is_empty() || !is_valid_email(&email) {
        return Err(AppError::BadRequest("Invalid email address".into()));
    }

    let user = db::users::get_user_by_email(&pool, &email)
        .await?
        .ok_or_else(|| AppError::BadRequest("No account found with this email".into()))?;

    if user.email_verified_at.is_some() {
        return Err(AppError::BadRequest("Email is already verified".into()));
    }

    let new_token = generate_verification_token();
    db::users::set_verification_token(&pool, user.id, &new_token).await?;

    let send_result = email::send_verification_email(
        &config,
        &user.email,
        &user.display_name,
        &new_token,
    )
    .await;

    match send_result {
        Ok(()) => tracing::info!(user_id = %user.id, "verification email resent"),
        Err(e) => tracing::warn!(user_id = %user.id, error = %e, "failed to resend verification email"),
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Verification email sent. Please check your inbox.",
    })))
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

    if user.email_verified_at.is_none() {
        return Err(AppError::BadRequest(
            "Please verify your email before signing in. Check your inbox or request a new verification email.".into(),
        ));
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
            if let Err(e) = storage.delete(&stored.storage_path).await {
                tracing::warn!("Failed to delete clip file {} during account deletion: {e}", stored.storage_path);
            }
            if let Some(thumb) = &stored.thumbnail_path {
                if let Err(e) = storage.delete(thumb).await {
                    tracing::warn!("Failed to delete thumbnail {thumb} during account deletion: {e}");
                }
            }
            if let Err(e) = db::clips::delete_clip(&pool, stored.id).await {
                tracing::warn!("Failed to delete clip {} from DB during account deletion: {e}", stored.id);
            }
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
