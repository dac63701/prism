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
    pub google_id: Option<String>,
    pub avatar_url: Option<String>,
    pub display_name: String,
    pub real_name: String,
    pub role: String,
    pub storage_used_bytes: i64,
    pub max_storage_bytes: i64,
    pub is_banned: bool,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub verification_token: Option<String>,
    pub verification_code: Option<String>,
    pub totp_secret: Option<String>,
    pub totp_enabled: bool,
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
    pub avatar_url: Option<String>,
    pub role: String,
    pub clip_count: i64,
    pub storage_used_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub is_banned: bool,
}

const USER_COLUMNS: &str = r#"id, email, password_hash, google_id, avatar_url, display_name, real_name, role::text as role,
           storage_used_bytes, max_storage_bytes, is_banned,
           email_verified_at, verification_token, verification_code,
           totp_secret, totp_enabled,
           created_at, updated_at"#;

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    display_name: &str,
    max_storage_bytes: i64,
    google_id: Option<&str>,
    avatar_url: Option<&str>,
    verification_token: Option<&str>,
    verification_code: Option<&str>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        &format!(
            r#"INSERT INTO users (email, password_hash, display_name, max_storage_bytes, google_id, avatar_url, verification_token, verification_code)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING {USER_COLUMNS}"#
        ),
    )
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .bind(max_storage_bytes)
    .bind(google_id)
    .bind(avatar_url)
    .bind(verification_token)
    .bind(verification_code)
    .fetch_one(pool)
    .await
}

pub async fn create_google_user(
    pool: &PgPool,
    email: &str,
    display_name: &str,
    max_storage_bytes: i64,
    google_id: &str,
    avatar_url: Option<&str>,
) -> Result<User, sqlx::Error> {
    create_user(
        pool,
        email,
        "",
        display_name,
        max_storage_bytes,
        Some(google_id),
        avatar_url,
        None,
        None,
    )
    .await
}

pub async fn get_user_by_google_id(
    pool: &PgPool,
    google_id: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        &format!(
            r#"SELECT {USER_COLUMNS}
               FROM users WHERE google_id = $1"#
        ),
    )
    .bind(google_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        &format!(
            r#"SELECT {USER_COLUMNS}
               FROM users WHERE email = $1"#
        ),
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}

pub async fn get_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        &format!(
            r#"SELECT {USER_COLUMNS}
               FROM users WHERE id = $1"#
        ),
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_user_by_display_name(
    pool: &PgPool,
    display_name: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        &format!(
            r#"SELECT {USER_COLUMNS}
               FROM users WHERE display_name = $1"#
        ),
    )
    .bind(display_name)
    .fetch_optional(pool)
    .await
}

pub async fn get_user_by_verification_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        &format!(
            r#"SELECT {USER_COLUMNS}
               FROM users WHERE verification_token = $1"#
        ),
    )
    .bind(token)
    .fetch_optional(pool)
    .await
}

pub async fn verify_email(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE users SET email_verified_at = NOW(), verification_token = NULL, verification_code = NULL, updated_at = NOW() WHERE id = $1"#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_verification_token(
    pool: &PgPool,
    user_id: Uuid,
    token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE users SET verification_token = $1, updated_at = NOW() WHERE id = $2"#,
    )
    .bind(token)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_verification_code(
    pool: &PgPool,
    user_id: Uuid,
    code: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE users SET verification_code = $1, updated_at = NOW() WHERE id = $2"#,
    )
    .bind(code)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_user_by_verification_code(
    pool: &PgPool,
    email: &str,
    code: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        &format!(
            r#"SELECT {USER_COLUMNS}
               FROM users
               WHERE email = $1 AND verification_code = $2"#
        ),
    )
    .bind(email)
    .bind(code)
    .fetch_optional(pool)
    .await
}

pub async fn set_totp_secret(
    pool: &PgPool,
    user_id: Uuid,
    secret: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE users SET totp_secret = $1, updated_at = NOW() WHERE id = $2"#,
    )
    .bind(secret)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn enable_totp(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE users SET totp_enabled = true, updated_at = NOW() WHERE id = $1"#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn disable_totp(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE users SET totp_enabled = false, totp_secret = NULL, updated_at = NOW() WHERE id = $1"#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
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

pub async fn update_user_display_name(
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

pub async fn update_user_profile(
    pool: &PgPool,
    id: Uuid,
    display_name: &str,
    real_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET display_name = $1, real_name = $2, updated_at = NOW() WHERE id = $3",
    )
    .bind(display_name)
    .bind(real_name)
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
        r#"SELECT u.id, u.email, u.display_name, u.avatar_url, u.role::text as role,
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

pub async fn update_user_avatar(
    pool: &PgPool,
    id: Uuid,
    avatar_url: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET avatar_url = $1, updated_at = NOW() WHERE id = $2")
        .bind(avatar_url)
        .bind(id)
    .execute(pool)
        .await?;
    Ok(())
}

pub async fn link_google_account(
    pool: &PgPool,
    id: Uuid,
    google_id: &str,
    avatar_url: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET google_id = $1, avatar_url = $2, password_hash = '', updated_at = NOW() WHERE id = $3",
    )
    .bind(google_id)
    .bind(avatar_url)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
