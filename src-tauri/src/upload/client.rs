//! HTTP upload client for sending clips to remote servers using reqwest.

use std::path::Path;

/// Upload a clip file to the given URL via multipart POST.
///
/// Returns the server response body on success.
pub async fn upload_clip(
    url: &str,
    file_path: &Path,
    api_token: Option<&str>,
) -> Result<String, UploadError> {
    let file_bytes = tokio::fs::read(file_path)
        .await
        .map_err(|e| UploadError::File(format!("Failed to read file: {e}")))?;

    let filename = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("clip.mp4")
        .to_string();

    let file_part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(filename);

    let form = reqwest::multipart::Form::new().part("clip", file_part);

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
