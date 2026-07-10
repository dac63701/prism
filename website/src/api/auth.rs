use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{extract::State, http::StatusCode, Json};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{jwt, AuthUser};
use crate::config::Config;
use crate::db;
use crate::errors::AppError;

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
    refresh_token: String,
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

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Password hash error: {e}")))?
        .to_string())
}

fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Invalid password hash: {e}")))?;
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

// ── Endpoints ──────────────────────────────────────────────────────

pub async fn register(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
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
        .unwrap_or_else(|| body.email.split('@').next().unwrap_or("User").to_string());

    let password_hash = hash_password(&body.password)?;
    let max_bytes = (config.default_max_storage_gb * 1_073_741_824) as i64;

    let user = db::users::create_user(&pool, &body.email, &password_hash, &display_name, max_bytes)
        .await?;

    let response = make_auth_response(&user, &config)?;

    tracing::info!(user_id = %user.id, "user_registered");
    log_activity(&pool, user.id, "user_registered", None).await;

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn login(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let user = db::users::get_user_by_email(&pool, &body.email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if user.is_banned {
        return Err(AppError::Forbidden);
    }

    if !verify_password(&body.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let response = make_auth_response(&user, &config)?;

    log_activity(&pool, user.id, "user_logged_in", None).await;

    Ok(Json(response))
}

pub async fn refresh(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let claims = jwt::verify_refresh_token(&body.refresh_token, &config.jwt_secret)?;

    let user = db::users::get_user_by_id(&pool, claims.sub)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if user.is_banned {
        return Err(AppError::Forbidden);
    }

    let response = make_auth_response(&user, &config)?;
    Ok(Json(response))
}

pub async fn me(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> Result<Json<UserResponse>, AppError> {
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
    _storage: State<crate::storage::local::LocalStorage>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = db::users::get_user_by_id(&pool, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let user_id = user.id;

    tokio::task::spawn_blocking(move || {
        let path = format!("clips/{}", user_id);
        let full_path = std::path::PathBuf::from(".").join(&path);
        let _ = std::fs::remove_dir_all(&full_path);
    });

    db::users::delete_user(&pool, user_id).await?;

    tracing::info!(user_id = %user_id, "user_deleted");

    Ok(Json(serde_json::json!({"status": "ok"})))
}

// ── API Keys ───────────────────────────────────────────────────────

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

// ── Activity Logging ───────────────────────────────────────────────

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

#[allow(dead_code)]
pub async fn log_activity_with_ip(
    pool: &PgPool,
    user_id: Uuid,
    action: &str,
    ip_address: &str,
    details: Option<&serde_json::Value>,
) {
    let result = sqlx::query(
        r#"INSERT INTO activity_logs (user_id, action, ip_address, details)
           VALUES ($1, $2::log_action, $3, $4::jsonb)"#,
    )
    .bind(user_id)
    .bind(action)
    .bind(ip_address)
    .bind(details.map(|v| v.to_string()))
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::warn!(%user_id, %action, error = %e, "Failed to log activity");
    }
}
