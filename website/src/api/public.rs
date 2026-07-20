use axum::{
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use sqlx::PgPool;

use crate::config::Config;
use crate::db;
use crate::errors::AppError;
use crate::storage::local::LocalStorage;
use crate::storage::StorageBackend;

pub async fn share_meta(
    State(pool): State<PgPool>,
    Path(share_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let clip = db::clips::get_clip_by_share_id(&pool, &share_id)
        .await?
        .ok_or(AppError::NotFound("Clip not found".into()))?;

    if clip.visibility == "private" {
        return Err(AppError::NotFound("Clip not found".into()));
    }

    let user = db::users::get_user_by_id(&pool, clip.user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    Ok(Json(serde_json::json!({
        "clip": {
            "id": clip.id,
            "title": clip.title,
            "game": clip.game,
            "duration_secs": clip.duration_secs,
            "width": clip.width,
            "height": clip.height,
            "size_bytes": clip.size_bytes,
            "codec": clip.codec,
            "visibility": clip.visibility,
            "created_at": clip.created_at.to_rfc3339(),
            "video_url": format!("/api/media/{}", clip.storage_path),
            "thumbnail_url": clip.thumbnail_path.as_ref().map(|p| format!("/api/media/{}", p)),
            "share_url": format!("/s/{}", clip.share_id),
        },
        "user": {
            "id": user.id,
            "display_name": user.display_name,
            "avatar_url": user.avatar_url,
        }
    })))
}

pub async fn profile_meta(
    State(pool): State<PgPool>,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = db::users::get_user_by_display_name(&pool, &username)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    let (clips, _) = db::clips::list_clips(
        &pool,
        Some(user.id),
        "",
        "",
        "",
        "created_at",
        "desc",
        1,
        24,
    )
    .await?;

    let public_clips: Vec<_> = clips
        .into_iter()
        .filter(|clip| clip.visibility != "private")
        .collect();

    Ok(Json(serde_json::json!({
        "user": {
            "id": user.id,
            "display_name": user.display_name,
            "avatar_url": user.avatar_url,
            "created_at": user.created_at.to_rfc3339(),
        },
        "clips": public_clips,
    })))
}

pub async fn serve_share_page(
    State(cfg): State<Config>,
    Path(share_id): Path<String>,
) -> Result<Redirect, AppError> {
    Ok(Redirect::temporary(&format!("{}/s/{}", cfg.public_url(), share_id)))
}

pub async fn serve_profile_page(
    State(cfg): State<Config>,
    Path(username): Path<String>,
) -> Result<Redirect, AppError> {
    Ok(Redirect::temporary(&format!("{}/u/{}", cfg.public_url(), username)))
}

pub async fn serve_media(
    State(storage): State<LocalStorage>,
    Path(path): Path<String>,
) -> Result<Response, AppError> {
    let data: Vec<u8> = storage.retrieve(&path).await?;

    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    let headers = [(header::CONTENT_TYPE, mime.to_string())];

    Ok((headers, data).into_response())
}
