use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub display_name: String,
    pub role: String,
    pub storage_used_bytes: i64,
    pub max_storage_bytes: i64,
    pub is_banned: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct UserWithStats {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub storage_used_bytes: i64,
    pub max_storage_bytes: i64,
    pub is_banned: bool,
    pub clip_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserListItem {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub clip_count: i64,
    pub storage_used_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub is_banned: bool,
}

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    display_name: &str,
    max_storage_bytes: i64,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"INSERT INTO users (email, password_hash, display_name, max_storage_bytes)
           VALUES ($1, $2, $3, $4)
           RETURNING id, email, password_hash, display_name, role,
                     storage_used_bytes, max_storage_bytes, is_banned,
                     created_at, updated_at"#,
    )
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .bind(max_storage_bytes)
    .fetch_one(pool)
    .await
}

pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"SELECT id, email, password_hash, display_name, role,
                  storage_used_bytes, max_storage_bytes, is_banned,
                  created_at, updated_at
           FROM users WHERE email = $1"#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}

pub async fn get_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"SELECT id, email, password_hash, display_name, role,
                  storage_used_bytes, max_storage_bytes, is_banned,
                  created_at, updated_at
           FROM users WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn update_user_password(
    pool: &PgPool,
    id: Uuid,
    new_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
        .bind(new_hash)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_user_profile(
    pool: &PgPool,
    id: Uuid,
    display_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET display_name = $1, updated_at = NOW() WHERE id = $2")
        .bind(display_name)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_user_role(pool: &PgPool, id: Uuid, role: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET role = $1::user_role, updated_at = NOW() WHERE id = $2")
        .bind(role)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_user_banned(pool: &PgPool, id: Uuid, banned: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET is_banned = $1, updated_at = NOW() WHERE id = $2")
        .bind(banned)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_user_max_storage(
    pool: &PgPool,
    id: Uuid,
    max_bytes: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET max_storage_bytes = $1, updated_at = NOW() WHERE id = $2")
        .bind(max_bytes)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn add_storage_used(pool: &PgPool, id: Uuid, bytes: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET storage_used_bytes = storage_used_bytes + $1 WHERE id = $2")
        .bind(bytes)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_users(
    pool: &PgPool,
    search: &str,
    page: i64,
    per_page: i64,
) -> Result<(Vec<UserListItem>, i64), sqlx::Error> {
    let offset = (page - 1) * per_page;
    let pattern = format!("%{}%", search);

    let total: i64 = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM users
           WHERE email ILIKE $1 OR display_name ILIKE $1"#,
    )
    .bind(&pattern)
    .fetch_one(pool)
    .await?;

    let users = sqlx::query_as::<_, UserListItem>(
        r#"SELECT u.id, u.email, u.display_name, u.role,
                  COALESCE(c.clip_count, 0) as clip_count,
                  u.storage_used_bytes, u.created_at, u.is_banned
           FROM users u
           LEFT JOIN (SELECT user_id, COUNT(*) as clip_count FROM clips GROUP BY user_id) c
             ON c.user_id = u.id
           WHERE u.email ILIKE $1 OR u.display_name ILIKE $1
           ORDER BY u.created_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(&pattern)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok((users, total))
}

pub async fn delete_user(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
