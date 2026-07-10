use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub async fn upload_clip(
    server_url: &str,
    api_key: &str,
    file_path: &Path,
    title: &str,
    game: &str,
    duration_secs: f64,
    width: u32,
    height: u32,
) -> Result<String, UploadError> {
    let file_bytes = tokio::fs::read(file_path)
        .await
        .map_err(|e| UploadError::File(format!("Failed to read file: {e}")))?;

    let filename = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("clip.mp4")
        .to_string();

    let url = format!("{}/api/clips/upload", server_url.trim_end_matches('/'));

    let file_part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(filename)
        .mime_str("video/mp4")
        .map_err(|e| UploadError::Http(format!("Mime error: {e}")))?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("title", title.to_string())
        .text("game", game.to_string())
        .text("duration_secs", duration_secs.to_string())
        .text("width", width.to_string())
        .text("height", height.to_string())
        .text("codec", "h264".to_string())
        .text("visibility", "unlisted".to_string());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| UploadError::Http(format!("Client error: {e}")))?;

    let req = client
        .post(&url)
        .multipart(form)
        .header("Authorization", format!("Bearer {api_key}"));

    let resp = req
        .send()
        .await
        .map_err(|e| UploadError::Http(format!("Request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(UploadError::Http(format!("HTTP {status}: {body}")));
    }

    resp.text()
        .await
        .map_err(|e| UploadError::Http(format!("Failed to read response: {e}")))
}

#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("File error: {0}")]
    File(String),
    #[error("HTTP error: {0}")]
    Http(String),
}
