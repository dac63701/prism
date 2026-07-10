#![allow(dead_code)]

use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;
use crate::errors::AppError;

pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn extract_key_prefix(key: &str) -> &str {
    if key.len() > 12 {
        &key[..12]
    } else {
        key
    }
}

pub async fn verify_api_key(pool: &PgPool, key: &str) -> Result<Uuid, AppError> {
    if !key.starts_with("prism_") {
        return Err(AppError::Unauthorized);
    }

    let key_body = &key[6..];
    let prefix = extract_key_prefix(key_body);
    let hash = hash_api_key(key);

    let stored = db::api_keys::get_api_key_by_prefix(pool, prefix)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if stored.is_revoked {
        return Err(AppError::Unauthorized);
    }

    if stored.key_hash != hash {
        return Err(AppError::Unauthorized);
    }

    db::api_keys::update_api_key_used(pool, stored.id).await?;

    Ok(stored.user_id)
}
