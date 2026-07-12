use std::io::Write;
use std::path::Path;

use reqwest::Body;
use serde::Deserialize;
use uuid::Uuid;

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

/// Build a multipart/form-data body manually.
///
/// reqwest's `multipart::Form` has formatting differences vs what multer 3.x
/// (used by axum 0.8) expects — notably it emits `Content-Transfer-Encoding`
/// headers that can cause parse failures.  This function constructs the body
/// with a bare-bones format that mirrors what browsers send.
fn build_multipart_body(
    boundary: &str,
    filename: &str,
    file_bytes: &[u8],
    metadata: &UploadMetadata,
) -> Vec<u8> {
    let mut body = Vec::new();

    // File part (CRLF line endings per RFC 2046)
    let _ = write!(body, "--{boundary}\r\n");
    let _ = write!(
        body,
        "Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n"
    );
    let _ = write!(body, "Content-Type: video/mp4\r\n");
    let _ = write!(body, "\r\n");
    body.extend_from_slice(file_bytes);
    let _ = write!(body, "\r\n");

    // Text fields
    let duration_secs_str = metadata.duration_secs.to_string();
    let width_str = metadata.width.to_string();
    let height_str = metadata.height.to_string();
    let text_fields: [(&str, &str); 7] = [
        ("title", metadata.title.as_str()),
        ("game", metadata.game.as_str()),
        ("duration_secs", &duration_secs_str),
        ("width", &width_str),
        ("height", &height_str),
        ("codec", metadata.codec.as_str()),
        ("visibility", "unlisted"),
    ];
    for (name, value) in &text_fields {
        let _ = write!(body, "--{boundary}\r\n");
        let _ = write!(body, "Content-Disposition: form-data; name=\"{name}\"\r\n");
        let _ = write!(body, "\r\n");
        let _ = write!(body, "{value}\r\n");
    }

    // Closing boundary
    let _ = write!(body, "--{boundary}--\r\n");

    body
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

    let boundary = Uuid::new_v4().to_string();
    let content_type = format!("multipart/form-data; boundary={boundary}");
    let body = build_multipart_body(&boundary, &filename, &file_bytes, metadata);

    let client = reqwest::Client::new();
    let mut req = client
        .post(url)
        .header("Content-Type", &content_type)
        .body(Body::from(body));

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_upload_file_not_found() {
        let result = upload_clip(
            "https://example.com/api/clips/upload",
            Path::new("/nonexistent/path.mp4"),
            Some("test_key"),
            &UploadMetadata {
                title: "test".into(),
                game: "Test".into(),
                duration_secs: 30.0,
                width: 1920,
                height: 1080,
                codec: "h264".into(),
                size_bytes: 1000,
            },
        )
        .await;

        assert!(result.is_err());
        match result {
            Err(UploadError::File(msg)) => {
                assert!(
                    msg.contains("Failed to read file"),
                    "expected file error, got: {msg}"
                );
            }
            other => panic!("expected File error, got: {other:?}"),
        }
    }
}
