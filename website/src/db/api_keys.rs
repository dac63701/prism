use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    #[serde(skip_serializing)]
    pub key_hash: String,
    pub key_prefix: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_revoked: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ApiKeyListItem {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub async fn insert_api_key(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    key_hash: &str,
    key_prefix: &str,
) -> Result<ApiKey, sqlx::Error> {
    sqlx::query_as::<_, ApiKey>(
        r#"INSERT INTO api_keys (user_id, name, key_hash, key_prefix)
           VALUES ($1, $2, $3, $4)
           RETURNING id, user_id, name, key_hash, key_prefix, last_used_at,
                     is_revoked, created_at"#,
    )
    .bind(user_id)
    .bind(name)
    .bind(key_hash)
    .bind(key_prefix)
    .fetch_one(pool)
    .await
}

pub async fn get_api_key_by_prefix(
    pool: &PgPool,
    prefix: &str,
) -> Result<Option<ApiKey>, sqlx::Error> {
    sqlx::query_as::<_, ApiKey>(
        r#"SELECT id, user_id, name, key_hash, key_prefix, last_used_at,
                  is_revoked, created_at
           FROM api_keys WHERE key_prefix = $1 AND is_revoked = false"#,
    )
    .bind(prefix)
    .fetch_optional(pool)
    .await
}

pub async fn update_api_key_used(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_api_keys(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<ApiKeyListItem>, sqlx::Error> {
    sqlx::query_as::<_, ApiKeyListItem>(
        r#"SELECT id, name, key_prefix, last_used_at, created_at
           FROM api_keys
           WHERE user_id = $1 AND is_revoked = false
           ORDER BY created_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn revoke_api_key(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE api_keys SET is_revoked = true WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn admin_revoke_api_key(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE api_keys SET is_revoked = true WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
