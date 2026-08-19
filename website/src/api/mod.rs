use axum::{extract::DefaultBodyLimit, routing, Router};

use crate::AppState;

pub mod admin;
pub mod auth;
pub mod clips;
pub mod public;
pub mod tags;

pub fn add_api_routes(router: Router<AppState>, upload_body_limit: usize) -> Router<AppState> {
    router
        .route("/api/health", routing::get(admin::health))
        .route("/api/auth/google", routing::get(auth::google_start))
        .route("/api/auth/google/callback", routing::get(auth::google_callback))
        .route("/api/auth/register", routing::post(auth::register))
        .route("/api/auth/verify-email", routing::get(auth::verify_email))
        .route("/api/auth/resend-verification", routing::post(auth::resend_verification))
        .route("/api/auth/verify-code", routing::post(auth::verify_code))
        .route("/api/auth/2fa/login", routing::post(auth::tfa_login))
        .route("/api/auth/2fa/setup", routing::post(auth::tfa_setup))
        .route("/api/auth/2fa/enable", routing::post(auth::tfa_enable))
        .route("/api/auth/2fa/disable", routing::post(auth::tfa_disable))
        .route("/api/auth/2fa/send-code", routing::post(auth::tfa_send_code))
        .route("/api/auth/2fa/send-code-login", routing::post(auth::tfa_send_code_login))
        .route("/api/auth/login", routing::post(auth::login))
        .route("/api/auth/refresh", routing::post(auth::refresh))
        .route("/api/auth/logout", routing::post(auth::logout))
        .route("/api/auth/desktop/success", routing::get(auth::desktop_success))
        .route("/api/auth/desktop/exchange", routing::post(auth::desktop_exchange))
        .route("/api/auth/desktop/poll", routing::get(auth::desktop_poll))
        .route("/api/auth/me", routing::get(auth::me))
        .route("/api/auth/me", routing::delete(auth::delete_account))
        .route(
            "/api/auth/change-password",
            routing::post(auth::change_password),
        )
        .route(
            "/api/auth/update-profile",
            routing::post(auth::update_profile),
        )
        .route(
            "/api/clips/upload",
            routing::post(clips::upload_clip).layer(DefaultBodyLimit::max(upload_body_limit)),
        )
        .route("/api/clips", routing::get(clips::list_clips))
        .route("/api/clips/{id}", routing::get(clips::get_clip))
        .route("/api/clips/{id}", routing::delete(clips::delete_clip))
        .route(
            "/api/clips/{id}/regenerate-share",
            routing::post(clips::regenerate_share),
        )
        .route(
            "/api/clips/{id}/visibility",
            routing::patch(clips::update_clip_visibility),
        )
        .route(
            "/api/clips/{id}/name",
            routing::patch(clips::update_clip_name),
        )
        .route("/api/s/{share_id}/meta", routing::get(public::share_meta))
        .route("/api/u/{username}", routing::get(public::profile_meta))
        .route("/api/media/{*path}", routing::get(public::serve_media))
        .route("/api/admin/users", routing::get(admin::list_users))
        .route("/api/admin/users/{id}", routing::get(admin::get_user))
        .route("/api/admin/users/{id}", routing::patch(admin::update_user))
        .route("/api/admin/users/{id}", routing::delete(admin::delete_user))
        .route("/api/admin/stats", routing::get(admin::get_stats))
        .route("/api/admin/clips", routing::get(admin::list_all_clips))
        .route(
            "/api/admin/clips/{id}",
            routing::delete(admin::admin_delete_clip),
        )
        .route("/api/admin/logs", routing::get(admin::get_logs))
        .route("/api/admin/config", routing::get(admin::get_config))
        .route("/api/admin/config", routing::put(admin::update_config))
        .route("/api/clips/{id}/tags", routing::get(tags::list_tags))
        .route("/api/clips/{id}/tags", routing::put(tags::set_tags))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::{IntoResponse, Response},
        routing::post,
        Router,
    };
    use tower::ServiceExt;

    const BOUNDARY: &str = "TestBoundary123";

    fn multipart_body(file_size: usize) -> Body {
        let mut body = Vec::new();
        body.extend_from_slice(
            format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.mp4\"\r\nContent-Type: video/mp4\r\n\r\n").as_bytes(),
        );
        body.extend(std::iter::repeat_n(0u8, file_size));
        body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
        Body::from(body)
    }

    fn multipart_request(file_size: usize) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/upload")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .body(multipart_body(file_size))
            .unwrap()
    }

    async fn handle(mut multipart: axum::extract::Multipart) -> Response {
        let mut total = 0usize;
        loop {
            match multipart.next_field().await {
                Ok(Some(mut field)) => loop {
                    match field.chunk().await {
                        Ok(Some(bytes)) => total += bytes.len(),
                        Ok(None) => break,
                        Err(_) => {
                            return (StatusCode::BAD_REQUEST, "multipart error").into_response()
                        }
                    }
                },
                Ok(None) => break,
                Err(_) => return (StatusCode::BAD_REQUEST, "multipart error").into_response(),
            }
        }
        axum::Json(serde_json::json!({ "bytes": total })).into_response()
    }

    fn upload_router() -> Router {
        Router::new().route(
            "/upload",
            post(handle).layer(DefaultBodyLimit::max(32 * 1024 * 1024)),
        )
    }

    fn upload_router_default_limit() -> Router {
        Router::new().route("/upload", post(handle))
    }

    /// A 3 MB body (over axum's default 2 MB Multipart limit) must parse once the
    /// route overrides DefaultBodyLimit — this is the upload bug that showed up as
    /// "Failed to read file: Error parsing `multipart/form-data` request".
    #[tokio::test]
    async fn test_large_multipart_with_override() {
        let response = upload_router()
            .oneshot(multipart_request(3 * 1024 * 1024))
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "large multipart body should parse with DefaultBodyLimit override"
        );
    }

    /// Without the override the default 2 MB limit rejects the same body.
    #[tokio::test]
    async fn test_large_multipart_default_limit_rejects() {
        let response = upload_router_default_limit()
            .oneshot(multipart_request(3 * 1024 * 1024))
            .await
            .unwrap();
        assert_ne!(
            response.status(),
            StatusCode::OK,
            "large multipart body should be rejected by the default 2 MB limit"
        );
    }
}
