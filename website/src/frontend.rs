use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::path::PathBuf;
use tower::ServiceExt;
use tower_http::services::ServeDir;

/// Serve the built React frontend.
/// Falls back to index.html for SPA routing (non-file routes).
#[derive(Clone)]
pub struct FrontendStatic {
    dir: PathBuf,
}

impl FrontendStatic {
    pub fn new(dir: &str) -> Self {
        Self {
            dir: PathBuf::from(dir),
        }
    }

    pub fn index_html_path(&self) -> PathBuf {
        self.dir.join("index.html")
    }

    pub async fn read_index_html(&self) -> std::io::Result<String> {
        tokio::fs::read_to_string(self.index_html_path()).await
    }

    pub async fn serve(&self, request: Request) -> Response {
        let path = request.uri().path().trim_start_matches('/');

        if path.contains('.') {
            let serve_dir = ServeDir::new(&self.dir)
                .append_index_html_on_directories(false)
                .precompressed_gzip()
                .precompressed_br()
                .precompressed_zstd();
            return match serve_dir.oneshot(request).await {
                Ok(resp) => resp.map(Body::new),
                Err(_) => StatusCode::NOT_FOUND.into_response(),
            };
        }

        match tokio::fs::read(self.index_html_path()).await {
            Ok(content) => {
                let mime = mime_guess::from_path("index.html").first_or_octet_stream();
                let headers = [(header::CONTENT_TYPE, mime.to_string())];
                (headers, content).into_response()
            }
            Err(e) => {
                tracing::warn!(
                    "Frontend index.html not found at {:?}: {e}",
                    self.index_html_path()
                );
                (
                    StatusCode::NOT_FOUND,
                    "Frontend not built. Run `cd frontend && npm run build` first.",
                )
                    .into_response()
            }
        }
    }
}
