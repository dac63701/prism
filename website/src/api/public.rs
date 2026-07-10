use axum::{
    extract::{Path, State},
    http::header,
    response::{Html, IntoResponse, Response},
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

    db::clips::increment_download_count(&pool, clip.id).await?;

    Ok(Json(serde_json::json!({
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
        "thumbnail_url": clip.thumbnail_path.map(|p| format!("/api/media/{}", p)),
    })))
}

pub async fn serve_share_page(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Path(share_id): Path<String>,
) -> Result<Response, AppError> {
    let clip = db::clips::get_clip_by_share_id(&pool, &share_id)
        .await?
        .ok_or(AppError::NotFound("Clip not found".into()))?;

    if clip.visibility == "private" {
        return Err(AppError::NotFound("Clip not found".into()));
    }

    let title = if clip.title.is_empty() {
        clip.original_filename.clone()
    } else {
        clip.title.clone()
    };

    let description = format!(
        "Game: {} · Duration: {:.0}s · {}MB",
        if clip.game.is_empty() { "Unknown" } else { &clip.game },
        clip.duration_secs,
        clip.size_bytes as f64 / 1_048_576.0,
    );

    let og_image = clip.thumbnail_path.as_ref().map(|p| {
        format!("{}/api/media/{}", config.public_url(), p)
    }).unwrap_or_default();

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} — Prism Clip</title>

    <!-- Open Graph -->
    <meta property="og:title" content="{}">
    <meta property="og:description" content="{}">
    <meta property="og:type" content="video.other">
    <meta property="og:image" content="{}">
    <meta property="og:url" content="{}/s/{}">
    <meta name="twitter:card" content="player">
    <meta name="twitter:title" content="{}">
    <meta name="twitter:description" content="{}">
    <meta name="twitter:image" content="{}">

    <link rel="stylesheet" href="/assets/index.css">
    <script type="module" src="/assets/index.js"></script>
</head>
<body>
    <div id="root" data-share-id="{}"></div>
</body>
</html>"#,
        title, title, description, og_image, config.public_url(), share_id,
        title, description, og_image, share_id,
    );

    Ok(Html(html).into_response())
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
