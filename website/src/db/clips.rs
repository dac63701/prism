use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Clip {
    pub id: Uuid,
    pub user_id: Uuid,
    pub original_filename: String,
    pub storage_path: String,
    pub thumbnail_path: Option<String>,
    pub share_id: String,
    pub title: String,
    pub game: String,
    pub duration_secs: f64,
    pub size_bytes: i64,
    pub width: i32,
    pub height: i32,
    pub codec: String,
    pub visibility: String,
    pub download_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ClipListItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub game: String,
    pub duration_secs: f64,
    pub size_bytes: i64,
    pub width: i32,
    pub height: i32,
    pub visibility: String,
    pub thumbnail_path: Option<String>,
    pub share_id: String,
    pub created_at: DateTime<Utc>,
    pub user_email: Option<String>,
    pub user_display_name: Option<String>,
}

pub async fn insert_clip(pool: &PgPool, clip: &Clip) -> Result<Clip, sqlx::Error> {
    sqlx::query_as::<_, Clip>(
        r#"INSERT INTO clips (
               id, user_id, original_filename, storage_path, thumbnail_path,
               share_id, title, game, duration_secs, size_bytes,
               width, height, codec, visibility
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14::clip_visibility)
           RETURNING id, user_id, original_filename, storage_path, thumbnail_path,
                      share_id, title, game, duration_secs, size_bytes,
                      width, height, codec, visibility::text as visibility,
                      download_count, created_at, updated_at"#,
    )
    .bind(clip.id)
    .bind(clip.user_id)
    .bind(&clip.original_filename)
    .bind(&clip.storage_path)
    .bind(&clip.thumbnail_path)
    .bind(&clip.share_id)
    .bind(&clip.title)
    .bind(&clip.game)
    .bind(clip.duration_secs)
    .bind(clip.size_bytes)
    .bind(clip.width)
    .bind(clip.height)
    .bind(&clip.codec)
    .bind(&clip.visibility)
    .fetch_one(pool)
    .await
}

pub async fn get_clip(pool: &PgPool, id: Uuid) -> Result<Option<Clip>, sqlx::Error> {
    sqlx::query_as::<_, Clip>(
        r#"SELECT id, user_id, original_filename, storage_path, thumbnail_path,
                  share_id, title, game, duration_secs, size_bytes,
                  width, height, codec, visibility::text as visibility,
                  download_count, created_at, updated_at
           FROM clips WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_clip_by_share_id(
    pool: &PgPool,
    share_id: &str,
) -> Result<Option<Clip>, sqlx::Error> {
    sqlx::query_as::<_, Clip>(
        r#"SELECT id, user_id, original_filename, storage_path, thumbnail_path,
                  share_id, title, game, duration_secs, size_bytes,
                  width, height, codec, visibility::text as visibility,
                  download_count, created_at, updated_at
           FROM clips WHERE share_id = $1"#,
    )
    .bind(share_id)
    .fetch_optional(pool)
    .await
}

pub async fn list_clips(
    pool: &PgPool,
    user_id: Option<Uuid>,
    search: &str,
    game: &str,
    _visibility: &str,
    sort_by: &str,
    sort_dir: &str,
    page: i64,
    per_page: i64,
) -> Result<(Vec<ClipListItem>, i64), sqlx::Error> {
    let offset = (page - 1) * per_page;
    let search_pattern = format!("%{}%", search);
    let game_pattern = if game.is_empty() {
        "%".into()
    } else {
        format!("%{}%", game)
    };

    let order_col = match sort_by {
        "size" => "c.size_bytes",
        "duration" => "c.duration_secs",
        "title" => "c.title",
        _ => "c.created_at",
    };
    let order_dir = if sort_dir.eq_ignore_ascii_case("asc") {
        "ASC"
    } else {
        "DESC"
    };
    let order_clause = format!("{} {}", order_col, order_dir);

    let total_sql = if let Some(_uid) = user_id {
        r#"SELECT COUNT(*) FROM clips c
           WHERE c.user_id = $1
             AND (c.title ILIKE $2 OR c.original_filename ILIKE $2)
             AND c.game ILIKE $3"#
            .to_string()
    } else {
        r#"SELECT COUNT(*) FROM clips c
           WHERE (c.title ILIKE $1 OR c.original_filename ILIKE $1)
             AND c.game ILIKE $2"#
            .to_string()
    };

    let list_sql = if let Some(_uid) = user_id {
        format!(
            r#"SELECT c.id, c.user_id, c.title, c.game, c.duration_secs, c.size_bytes,
                       c.width, c.height, c.visibility::text as visibility, c.thumbnail_path,
                       c.share_id, c.created_at,
                       NULL as user_email,
                       NULL as user_display_name
                FROM clips c
                WHERE c.user_id = $1
                  AND (c.title ILIKE $2 OR c.original_filename ILIKE $2)
                  AND c.game ILIKE $3
                ORDER BY {}
                LIMIT $4 OFFSET $5"#,
            order_clause
        )
    } else {
        format!(
            r#"SELECT c.id, c.user_id, c.title, c.game, c.duration_secs, c.size_bytes,
                       c.width, c.height, c.visibility::text as visibility, c.thumbnail_path,
                       c.share_id, c.created_at, u.email as user_email, u.display_name as user_display_name
               FROM clips c
               LEFT JOIN users u ON u.id = c.user_id
               WHERE (c.title ILIKE $1 OR c.original_filename ILIKE $1)
                 AND c.game ILIKE $2
               ORDER BY {}
               LIMIT $3 OFFSET $4"#,
            order_clause
        )
    };

    let (total,): (i64,) = if let Some(uid) = user_id {
        sqlx::query_as(&total_sql)
            .bind(uid)
            .bind(&search_pattern)
            .bind(&game_pattern)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_as(&total_sql)
            .bind(&search_pattern)
            .bind(&game_pattern)
            .fetch_one(pool)
            .await?
    };

    let clips = if let Some(uid) = user_id {
        sqlx::query_as::<_, ClipListItem>(&list_sql)
            .bind(uid)
            .bind(&search_pattern)
            .bind(&game_pattern)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool)
            .await?
    } else {
        sqlx::query_as::<_, ClipListItem>(&list_sql)
            .bind(&search_pattern)
            .bind(&game_pattern)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool)
            .await?
    };

    Ok((clips, total))
}

pub async fn update_clip_visibility(
    pool: &PgPool,
    id: Uuid,
    visibility: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE clips SET visibility = $1::clip_visibility, updated_at = NOW() WHERE id = $2",
    )
    .bind(visibility)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_clip_title(
    pool: &PgPool,
    id: Uuid,
    title: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE clips SET title = $1, updated_at = NOW() WHERE id = $2")
        .bind(title)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn regenerate_share_id(pool: &PgPool, id: Uuid) -> Result<String, sqlx::Error> {
    use rand::Rng;
    let share_id: String = (0..6)
        .map(|_| format!("{:02x}", rand::thread_rng().gen::<u8>()))
        .collect();

    sqlx::query_scalar::<_, String>(
        r#"UPDATE clips SET share_id = $1, updated_at = NOW()
           WHERE id = $2
           RETURNING share_id"#,
    )
    .bind(share_id)
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn delete_clip(pool: &PgPool, id: Uuid) -> Result<Option<Clip>, sqlx::Error> {
    let clip = sqlx::query_as::<_, Clip>(
        r#"DELETE FROM clips WHERE id = $1
           RETURNING id, user_id, original_filename, storage_path, thumbnail_path,
                      share_id, title, game, duration_secs, size_bytes,
                      width, height, codec, visibility::text as visibility,
                      download_count, created_at, updated_at"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(clip)
}

pub async fn increment_download_count(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE clips SET download_count = download_count + 1 WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_server_stats(pool: &PgPool) -> Result<(i64, i64, i64, i64, i64), sqlx::Error> {
    let row = sqlx::query_as::<_, (Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<i64>)>(
        r#"SELECT
               (SELECT COUNT(*) FROM users) as total_users,
               (SELECT COUNT(*) FROM clips) as total_clips,
               (SELECT COALESCE(SUM(size_bytes), 0)::bigint FROM clips) as total_bytes,
               (SELECT COUNT(*) FROM clips WHERE created_at >= CURRENT_DATE) as uploads_today,
               (SELECT COUNT(*) FROM clips WHERE created_at >= CURRENT_DATE - INTERVAL '7 days') as uploads_week
           FROM (SELECT 1) t"#,
    )
    .fetch_one(pool)
    .await?;

    Ok((
        row.0.unwrap_or(0),
        row.1.unwrap_or(0),
        row.2.unwrap_or(0),
        row.3.unwrap_or(0),
        row.4.unwrap_or(0),
    ))
}
