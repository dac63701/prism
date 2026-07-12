use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{ApiKeyOrJwtAuth, AuthUser};
use crate::config::Config;
use crate::db;
use crate::db::tags;
use crate::errors::AppError;
use crate::storage::local::LocalStorage;
use crate::storage::StorageBackend;

#[derive(Deserialize)]
pub struct ListClipsQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    search: Option<String>,
    game: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateClipRequest {
    title: Option<String>,
    game: Option<String>,
    visibility: Option<String>,
}

pub async fn upload_clip(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    State(storage): State<LocalStorage>,
    ApiKeyOrJwtAuth(auth): ApiKeyOrJwtAuth,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let user = db::users::get_user_by_id(&pool, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let mut file_data: Vec<u8> = Vec::new();
    let mut original_filename = String::new();
    let mut title = String::new();
    let mut game = String::new();
    let mut duration_secs: f64 = 0.0;
    let mut width: i32 = 0;
    let mut height: i32 = 0;
    let mut codec = String::new();
    let mut visibility = "unlisted".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                original_filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "clip.mp4".to_string());
                file_data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Failed to read file: {e}")))?
                    .to_vec();
            }
            "title" => title = field.text().await.unwrap_or_default(),
            "game" => game = field.text().await.unwrap_or_default(),
            "duration_secs" => {
                duration_secs = field
                    .text()
                    .await
                    .unwrap_or_default()
                    .parse()
                    .unwrap_or(0.0)
            }
            "width" => width = field.text().await.unwrap_or_default().parse().unwrap_or(0),
            "height" => height = field.text().await.unwrap_or_default().parse().unwrap_or(0),
            "codec" => codec = field.text().await.unwrap_or_default(),
            "visibility" => {
                let v = field.text().await.unwrap_or_default();
                if matches!(v.as_str(), "public" | "private" | "unlisted") {
                    visibility = v;
                }
            }
            _ => {}
        }
    }

    if file_data.is_empty() {
        return Err(AppError::BadRequest("No file provided".into()));
    }

    let max_upload = config.max_upload_size_mb * 1_024 * 1_024;
    if file_data.len() as u64 > max_upload {
        return Err(AppError::UploadTooLarge);
    }

    if user.storage_used_bytes + file_data.len() as i64 > user.max_storage_bytes {
        return Err(AppError::StorageExceeded);
    }

    let clip_id = Uuid::new_v4();
    let storage_path = format!("clips/{}/{}.mp4", auth.user_id, clip_id);

    storage.store(&storage_path, &file_data).await?;

    // Generate thumbnail
    let thumb_storage_path = format!("thumbs/{}/{}.jpg", auth.user_id, clip_id);
    let thumbnail_path = {
        let tmp_video = std::env::temp_dir().join(format!("prism_{}.mp4", clip_id));
        let tmp_thumb = std::env::temp_dir().join(format!("prism_{}_thumb.jpg", clip_id));

        // Write video to temp file
        let _ = tokio::fs::write(&tmp_video, &file_data).await;

        // Generate thumbnail (ffmpeg or pattern fallback)
        let _ = crate::thumbnail::generate_thumbnail(&tmp_video, &tmp_thumb, 320);

        // Read and store thumbnail
        if let Ok(thumb_data) = tokio::fs::read(&tmp_thumb).await {
            let _ = storage.store(&thumb_storage_path, &thumb_data).await;
        }

        // Cleanup temp files
        let _ = tokio::fs::remove_file(&tmp_video).await;
        let _ = tokio::fs::remove_file(&tmp_thumb).await;

        // Only set thumbnail_path if the file was stored successfully
        if storage.exists(&thumb_storage_path).await.unwrap_or(false) {
            Some(thumb_storage_path)
        } else {
            None
        }
    };

    let share_id: String = (0..12)
        .map(|_| format!("{:x}", rand::random::<u8>()))
        .collect();

    let clip = db::clips::Clip {
        id: clip_id,
        user_id: auth.user_id,
        original_filename,
        storage_path,
        thumbnail_path,
        share_id,
        title,
        game,
        duration_secs,
        size_bytes: file_data.len() as i64,
        width,
        height,
        codec,
        visibility,
        download_count: 0,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    db::clips::insert_clip(&pool, &clip).await?;
    db::users::add_storage_used(&pool, auth.user_id, clip.size_bytes).await?;

    crate::api::auth::log_activity(&pool, auth.user_id, "clip_uploaded", None).await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": clip.id,
            "share_url": format!("/s/{}", clip.share_id),
            "title": clip.title,
            "game": clip.game,
            "duration_secs": clip.duration_secs,
            "size_bytes": clip.size_bytes,
            "created_at": clip.created_at.to_rfc3339(),
        })),
    ))
}

pub async fn list_clips(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Query(query): Query<ListClipsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(100).max(1);
    let search = query.search.unwrap_or_default();
    let game = query.game.unwrap_or_default();
    let sort_by = query.sort_by.unwrap_or_else(|| "created_at".into());
    let sort_dir = query.sort_dir.unwrap_or_else(|| "desc".into());

    let (clips, total) = db::clips::list_clips(
        &pool,
        Some(auth.user_id),
        &search,
        &game,
        "",
        &sort_by,
        &sort_dir,
        page,
        per_page,
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

pub async fn get_clip(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(clip_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let clip = db::clips::get_clip(&pool, clip_id)
        .await?
        .ok_or(AppError::NotFound("Clip not found".into()))?;

    if clip.user_id != auth.user_id && auth.role != "admin" {
        return Err(AppError::NotFound("Clip not found".into()));
    }

    let tags_list = tags::get_tags_for_clip(&pool, clip_id).await?;

    Ok(Json(serde_json::json!({
        "id": clip.id,
        "title": clip.title,
        "game": clip.game,
        "tags": tags_list,
        "duration_secs": clip.duration_secs,
        "size_bytes": clip.size_bytes,
        "width": clip.width,
        "height": clip.height,
        "codec": clip.codec,
        "visibility": clip.visibility,
        "share_url": format!("/s/{}", clip.share_id),
        "original_filename": clip.original_filename,
        "download_count": clip.download_count,
        "created_at": clip.created_at.to_rfc3339(),
        "updated_at": clip.updated_at.to_rfc3339(),
        "video_url": format!("/api/media/{}", clip.storage_path),
        "thumbnail_url": clip.thumbnail_path.as_ref().map(|p| format!("/api/media/{}", p)),
    })))
}

pub async fn update_clip(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(clip_id): Path<Uuid>,
    Json(body): Json<UpdateClipRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let clip = db::clips::get_clip(&pool, clip_id)
        .await?
        .ok_or(AppError::NotFound("Clip not found".into()))?;

    if clip.user_id != auth.user_id && auth.role != "admin" {
        return Err(AppError::NotFound("Clip not found".into()));
    }

    let new_title = body.title.as_deref().unwrap_or(&clip.title);
    let new_game = body.game.as_deref().unwrap_or(&clip.game);
    let new_visibility = body.visibility.as_deref().unwrap_or(&clip.visibility);

    if !matches!(new_visibility, "public" | "private" | "unlisted") {
        return Err(AppError::BadRequest("Invalid visibility".into()));
    }

    db::clips::update_clip_metadata(&pool, clip_id, new_title, new_game, new_visibility).await?;

    let updated = db::clips::get_clip(&pool, clip_id)
        .await?
        .ok_or(AppError::Internal("Failed to fetch updated clip".into()))?;

    Ok(Json(serde_json::json!({
        "id": updated.id,
        "title": updated.title,
        "game": updated.game,
        "visibility": updated.visibility,
        "updated_at": updated.updated_at.to_rfc3339(),
    })))
}

pub async fn delete_clip(
    State(pool): State<PgPool>,
    State(storage): State<LocalStorage>,
    auth: AuthUser,
    Path(clip_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let clip = db::clips::get_clip(&pool, clip_id)
        .await?
        .ok_or(AppError::NotFound("Clip not found".into()))?;

    if clip.user_id != auth.user_id && auth.role != "admin" {
        return Err(AppError::NotFound("Clip not found".into()));
    }

    if let Some(thumb) = &clip.thumbnail_path {
        let _ = storage.delete(thumb).await;
    }

    let _ = storage.delete(&clip.storage_path).await;

    db::clips::delete_clip(&pool, clip_id).await?;
    db::users::add_storage_used(&pool, auth.user_id, -clip.size_bytes).await?;

    crate::api::auth::log_activity(&pool, auth.user_id, "clip_deleted", None).await;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn regenerate_share(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(clip_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let clip = db::clips::get_clip(&pool, clip_id)
        .await?
        .ok_or(AppError::NotFound("Clip not found".into()))?;

    if clip.user_id != auth.user_id {
        return Err(AppError::NotFound("Clip not found".into()));
    }

    let new_share_id = db::clips::regenerate_share_id(&pool, clip_id).await?;

    Ok(Json(serde_json::json!({
        "share_id": new_share_id,
        "share_url": format!("/s/{}", new_share_id),
    })))
}
