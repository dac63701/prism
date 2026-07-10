#![allow(dead_code)]

use serde::Serialize;
use sqlx::{FromRow, PgPool};

use crate::errors::AppError;

#[derive(Debug, Serialize, FromRow)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_all(pool: &PgPool) -> Result<Vec<ConfigEntry>, AppError> {
    let rows = sqlx::query_as::<_, ConfigEntry>(
        "SELECT key, value, updated_at FROM server_config ORDER BY key",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(rows)
}

pub async fn get_value(pool: &PgPool, key: &str) -> Result<Option<String>, AppError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM server_config WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(row.map(|r| r.0))
}

pub async fn set_value(pool: &PgPool, key: &str, value: &str) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO server_config (key, value, updated_at) VALUES ($1, $2, NOW())
         ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}
