use axum::{
    extract::{Path, State},
    http::header,
    response::{Html, IntoResponse, Response},
    Json,
};
use sqlx::PgPool;

use crate::db;
use crate::db::tags;
use crate::errors::AppError;
use crate::AppState;
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
    let tags_list = tags::get_tags_for_clip(&pool, clip.id).await?;

    Ok(Json(serde_json::json!({
        "id": clip.id,
        "title": clip.title,
        "game": clip.game,
        "tags": tags_list,
        "duration_secs": clip.duration_secs,
        "width": clip.width,
        "height": clip.height,
        "size_bytes": clip.size_bytes,
        "codec": clip.codec,
        "visibility": clip.visibility,
        "created_at": clip.created_at.to_rfc3339(),
        "thumbnail_url": clip.thumbnail_path.as_ref().map(|p| format!("/api/media/{}", p)),
    })))
}

pub async fn serve_share_page(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
) -> Result<Response, AppError> {
    let clip = db::clips::get_clip_by_share_id(&state.pool, &share_id)
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
        if clip.game.is_empty() {
            "Unknown"
        } else {
            &clip.game
        },
        clip.duration_secs,
        clip.size_bytes as f64 / 1_048_576.0,
    );

    let og_image = clip
        .thumbnail_path
        .as_ref()
        .map(|p| format!("{}/api/media/{}", state.config.public_url(), p))
        .unwrap_or_default();

    let index_html = state.frontend.read_index_html().await.map_err(|e| {
        AppError::Internal(format!(
            "Frontend index.html not found at {:?}: {e}",
            state.frontend.index_html_path()
        ))
    })?;

    let title = escape_html(&title);
    let description = escape_html(&description);
    let og_image = escape_html(&og_image);
    let public_url = escape_html(&state.config.public_url());
    let share_id = escape_html(&share_id);
    let html = replace_title(index_html, &format!("{title} — Prism Clip"));

    let head_snippet = format!(
        r#"<meta property="og:title" content="{title}">
    <meta property="og:description" content="{description}">
    <meta property="og:type" content="video.other">
    <meta property="og:image" content="{og_image}">
    <meta property="og:url" content="{public_url}/s/{share_id}">
    <meta name="twitter:card" content="player">
    <meta name="twitter:title" content="{title}">
    <meta name="twitter:description" content="{description}">
    <meta name="twitter:image" content="{og_image}">"#
    );

    let html = inject_into_head(html, &head_snippet);

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

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn inject_into_head(html: String, snippet: &str) -> String {
    html.replacen("</head>", &format!("\n    {snippet}\n</head>"), 1)
}

fn replace_title(html: String, title: &str) -> String {
    if let (Some(start), Some(end)) = (html.find("<title>"), html.find("</title>")) {
        let mut updated = String::with_capacity(html.len() + title.len());
        updated.push_str(&html[..start]);
        updated.push_str("<title>");
        updated.push_str(title);
        updated.push_str("</title>");
        updated.push_str(&html[end + "</title>".len()..]);
        updated
    } else {
        html
    }
}
