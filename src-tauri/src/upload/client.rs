use std::path::Path;

use serde::Deserialize;

use super::queue::UploadMetadata;

#[derive(Debug, Deserialize)]
pub struct UploadResponse {
    pub id: String,
    pub share_url: String,
    pub title: String,
    pub game: Option<String>,
    pub duration_secs: f64,
    pub size_bytes: i64,
    pub created_at: String,
}

/// Upload a clip file with metadata to the given URL via multipart POST.
pub async fn upload_clip(
    url: &str,
    file_path: &Path,
    api_token: Option<&str>,
    metadata: &UploadMetadata,
) -> Result<UploadResponse, UploadError> {
    let file_bytes = tokio::fs::read(file_path)
        .await
        .map_err(|e| UploadError::File(format!("Failed to read file: {e}")))?;

    let filename = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("clip.mp4")
        .to_string();

    let file_part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(filename.clone())
        .mime_str("video/mp4")
        .map_err(|e| UploadError::Http(format!("MIME error: {e}")))?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("title", metadata.title.clone())
        .text("game", metadata.game.clone())
        .text("duration_secs", metadata.duration_secs.to_string())
        .text("width", metadata.width.to_string())
        .text("height", metadata.height.to_string())
        .text("codec", metadata.codec.clone())
        .text("visibility", "unlisted");

    let client = reqwest::Client::new();
    let mut req = client.post(url).multipart(form);

    if let Some(token) = api_token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| UploadError::Http(format!("Request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(UploadError::Http(format!("HTTP {status}: {body}")));
    }

    resp.json::<UploadResponse>()
        .await
        .map_err(|e| UploadError::Http(format!("Failed to decode response: {e}")))
}

#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("File error: {0}")]
    File(String),
    #[error("HTTP error: {0}")]
    Http(String),
}
