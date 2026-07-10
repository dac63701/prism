use axum::extract::{Path, State};
use axum::{http::StatusCode, Json};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db;
use crate::errors::AppError;

#[derive(Deserialize)]
pub struct SetTagsRequest {
    tags: Vec<String>,
}

pub async fn list_tags(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(clip_id): Path<Uuid>,
) -> Result<Json<Vec<String>>, AppError> {
    let clip = db::clips::get_clip(&pool, clip_id)
        .await?
        .ok_or(AppError::NotFound("Clip not found".into()))?;

    if clip.user_id != auth.user_id && auth.role != "admin" {
        return Err(AppError::NotFound("Clip not found".into()));
    }

    let tags = db::tags::get_tags_for_clip(&pool, clip_id).await?;
    Ok(Json(tags))
}

pub async fn set_tags(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(clip_id): Path<Uuid>,
    Json(body): Json<SetTagsRequest>,
) -> Result<StatusCode, AppError> {
    let clip = db::clips::get_clip(&pool, clip_id)
        .await?
        .ok_or(AppError::NotFound("Clip not found".into()))?;

    if clip.user_id != auth.user_id && auth.role != "admin" {
        return Err(AppError::NotFound("Clip not found".into()));
    }

    db::tags::set_tags(&pool, clip_id, &body.tags).await?;
    Ok(StatusCode::NO_CONTENT)
}
