use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::time::Duration;
use tokio::time::timeout;

pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
pub const UPLOAD_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

pub async fn timeout_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    let path = request.uri().path();
    let timeout_duration = if path.starts_with("/api/clips/upload") {
        UPLOAD_REQUEST_TIMEOUT
    } else {
        DEFAULT_REQUEST_TIMEOUT
    };

    match timeout(timeout_duration, next.run(request)).await {
        Ok(response) => Ok(response),
        Err(_) => Err((StatusCode::REQUEST_TIMEOUT, "Request timed out")),
    }
}
