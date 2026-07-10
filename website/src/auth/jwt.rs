use axum::http::request::Parts;
use axum::http::header;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: Uuid,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: Uuid,
    pub exp: usize,
    pub iat: usize,
    pub typ: String,
}

pub fn create_access_token(user_id: Uuid, role: &str, secret: &str) -> Result<String, AppError> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = AccessClaims {
        sub: user_id,
        role: role.to_string(),
        exp: now + 900,
        iat: now,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("JWT encode error: {e}")))
}

pub fn create_refresh_token(user_id: Uuid, secret: &str) -> Result<String, AppError> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = RefreshClaims {
        sub: user_id,
        exp: now + 2_592_000,
        iat: now,
        typ: "refresh".into(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("JWT encode error: {e}")))
}

pub fn verify_access_token(token: &str, secret: &str) -> Result<AccessClaims, AppError> {
    decode::<AccessClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|d| d.claims)
    .map_err(|_| AppError::Unauthorized)
}

pub fn verify_refresh_token(token: &str, secret: &str) -> Result<RefreshClaims, AppError> {
    let claims = decode::<RefreshClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|d| d.claims)
    .map_err(|_| AppError::Unauthorized)?;

    if claims.typ != "refresh" {
        return Err(AppError::Unauthorized);
    }
    Ok(claims)
}

pub fn extract_bearer_token(parts: &Parts) -> Option<String> {
    parts.headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}
