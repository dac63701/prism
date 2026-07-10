use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::config::Config;
use crate::db;
use crate::errors::AppError;
use crate::storage::local::LocalStorage;
use crate::storage::StorageBackend;

#[derive(Deserialize)]
pub struct ListUsersQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    search: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    role: Option<String>,
    is_banned: Option<bool>,
    max_storage_gb: Option<u64>,
}

#[derive(Deserialize)]
pub struct AdminListClipsQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    search: Option<String>,
    game: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
}

#[derive(Deserialize)]
pub struct LogsQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    action: Option<String>,
    level: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    max_upload_size_mb: Option<u64>,
    default_max_storage_gb: Option<u64>,
    rate_limit_per_min: Option<u64>,
}

// ── User Management ────────────────────────────────────────────────

pub async fn list_users(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(100).max(1);
    let search = query.search.unwrap_or_default();

    let (users, total) = db::users::list_users(&pool, &search, page, per_page).await?;

    Ok(Json(serde_json::json!({
        "users": users,
        "total": total,
        "page": page,
        "per_page": per_page,
        "total_pages": (total as f64 / per_page as f64).ceil() as i64,
    })))
}

pub async fn get_user(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = db::users::get_user_by_id(&pool, user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    let (clips, _) = db::clips::list_clips(
        &pool,
        Some(user_id), "", "", "", "created_at", "desc", 1, 1000,
    ).await?;

    let total_storage: i64 = clips.iter().map(|c| c.size_bytes).sum();

    Ok(Json(serde_json::json!({
        "id": user.id,
        "email": user.email,
        "display_name": user.display_name,
        "role": user.role,
        "storage_used_bytes": total_storage,
        "max_storage_bytes": user.max_storage_bytes,
        "is_banned": user.is_banned,
        "clip_count": clips.len(),
        "created_at": user.created_at.to_rfc3339(),
        "updated_at": user.updated_at.to_rfc3339(),
    })))
}

pub async fn update_user(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(role) = &body.role {
        if !matches!(role.as_str(), "user" | "admin") {
            return Err(AppError::BadRequest("Invalid role".into()));
        }
        db::users::update_user_role(&pool, user_id, role).await?;
        tracing::info!(user_id = %user_id, new_role = %role, "admin_role_changed");
    }

    if let Some(banned) = body.is_banned {
        db::users::set_user_banned(&pool, user_id, banned).await?;
        if banned {
            tracing::info!(user_id = %user_id, "admin_user_banned");
        } else {
            tracing::info!(user_id = %user_id, "admin_user_unbanned");
        }
    }

    if let Some(gb) = body.max_storage_gb {
        let max_bytes = (gb * 1_073_741_824) as i64;
        db::users::set_user_max_storage(&pool, user_id, max_bytes).await?;
    }

    Ok(Json(serde_json::json!({"status": "ok"})))
}

pub async fn delete_user(
    State(pool): State<PgPool>,
    State(storage): State<LocalStorage>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (clips, _) = db::clips::list_clips(
        &pool,
        Some(user_id), "", "", "", "created_at", "desc", 1, 1000,
    ).await?;

    for clip in &clips {
        let clip_detailed = db::clips::get_clip(&pool, clip.id).await?;
        if let Some(c) = clip_detailed {
            let _ = storage.delete(&c.storage_path).await;
            if let Some(thumb) = &c.thumbnail_path {
                let _ = storage.delete(thumb).await;
            }
            let _ = db::clips::delete_clip(&pool, c.id).await;
        }
    }

    db::users::delete_user(&pool, user_id).await?;
    tracing::info!(user_id = %user_id, "admin_user_deleted");

    Ok(Json(serde_json::json!({"status": "ok"})))
}

// ── Server Stats ───────────────────────────────────────────────────

pub async fn get_stats(
    State(pool): State<PgPool>,
    _admin: AdminUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let (total_users, total_clips, total_bytes, uploads_today, uploads_week) =
        db::clips::get_server_stats(&pool).await?;

    Ok(Json(serde_json::json!({
        "total_users": total_users,
        "total_clips": total_clips,
        "total_storage_bytes": total_bytes,
        "total_storage_gb": (total_bytes as f64 / 1_073_741_824.0).round() * 100.0 / 100.0,
        "uploads_today": uploads_today,
        "uploads_this_week": uploads_week,
    })))
}

// ── Admin Clip Management ──────────────────────────────────────────

pub async fn list_all_clips(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Query(query): Query<AdminListClipsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(100).max(1);
    let search = query.search.unwrap_or_default();
    let game = query.game.unwrap_or_default();
    let sort_by = query.sort_by.unwrap_or_else(|| "created_at".into());
    let sort_dir = query.sort_dir.unwrap_or_else(|| "desc".into());

    let (clips, total) = db::clips::list_clips(
        &pool, None, &search, &game, "", &sort_by, &sort_dir, page, per_page,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "clips": clips,
        "total": total,
        "page": page,
        "per_page": per_page,
        "total_pages": (total as f64 / per_page as f64).ceil() as i64,
    })))
}

pub async fn admin_delete_clip(
    State(pool): State<PgPool>,
    State(storage): State<LocalStorage>,
    _admin: AdminUser,
    Path(clip_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let clip = db::clips::get_clip(&pool, clip_id)
        .await?
        .ok_or(AppError::NotFound("Clip not found".into()))?;

    if let Some(thumb) = &clip.thumbnail_path {
        let _ = storage.delete(thumb).await;
    }
    let _ = storage.delete(&clip.storage_path).await;

    db::users::add_storage_used(&pool, clip.user_id, -clip.size_bytes).await?;
    db::clips::delete_clip(&pool, clip_id).await?;

    Ok(Json(serde_json::json!({"status": "ok"})))
}

// ── Activity Logs ──────────────────────────────────────────────────

pub async fn get_logs(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Query(query): Query<LogsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(100).min(200).max(1);
    let offset = (page - 1) * per_page;

    let mut where_clauses = Vec::new();
    if let Some(ref action) = query.action {
        where_clauses.push(format!("action = '{}'::log_action", action));
    }
    if let Some(ref level) = query.level {
        where_clauses.push(format!("level = '{}'::log_level", level));
    }
    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM activity_logs {}", where_sql);
    let total: (i64,) = sqlx::query_as(&count_sql)
        .fetch_one(&pool)
        .await
        .unwrap_or((0,));

    let query_sql = format!(
        "SELECT id, user_id, action as \"action\", level as \"level\", ip_address, details, created_at
         FROM activity_logs {}
         ORDER BY created_at DESC
         LIMIT {} OFFSET {}",
        where_sql, per_page, offset
    );

    #[derive(serde::Serialize, sqlx::FromRow)]
    struct LogEntry {
        id: Uuid,
        user_id: Option<Uuid>,
        action: String,
        level: String,
        ip_address: Option<String>,
        details: Option<serde_json::Value>,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let logs: Vec<LogEntry> = sqlx::query_as(&query_sql)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    Ok(Json(serde_json::json!({
        "logs": logs,
        "total": total.0,
        "page": page,
        "per_page": per_page,
    })))
}

// ── Health ─────────────────────────────────────────────────────────

pub async fn health(
    State(pool): State<PgPool>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    })))
}
