use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use tower::ServiceExt;
use tower_http::services::ServeDir;
use std::path::PathBuf;

/// Serve the built React frontend.
/// Falls back to index.html for SPA routing (non-file routes).
pub struct FrontendStatic {
    dir: PathBuf,
}

impl FrontendStatic {
    pub fn new(dir: &str) -> Self {
        Self {
            dir: PathBuf::from(dir),
        }
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

        match tokio::fs::read(self.dir.join("index.html")).await {
            Ok(content) => {
                let mime = mime_guess::from_path("index.html").first_or_octet_stream();
                let headers = [(header::CONTENT_TYPE, mime.to_string())];
                (headers, content).into_response()
            }
            Err(_) => (
                StatusCode::NOT_FOUND,
                "Frontend not built. Run `cd frontend && npm run build` first.",
            )
                .into_response(),
        }
    }
}
