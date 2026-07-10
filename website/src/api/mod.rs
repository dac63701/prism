use axum::{Router, routing};

use crate::AppState;

pub mod admin;
pub mod auth;
pub mod clips;
pub mod public;

pub fn add_api_routes(router: Router<AppState>) -> Router<AppState> {
    router
        .route("/api/health", routing::get(admin::health))
        .route("/api/auth/register", routing::post(auth::register))
        .route("/api/auth/login", routing::post(auth::login))
        .route("/api/auth/refresh", routing::post(auth::refresh))
        .route("/api/auth/me", routing::get(auth::me))
        .route("/api/auth/me", routing::delete(auth::delete_account))
        .route("/api/auth/change-password", routing::post(auth::change_password))
        .route("/api/auth/update-profile", routing::post(auth::update_profile))
        .route("/api/auth/api-keys", routing::get(auth::list_api_keys))
        .route("/api/auth/api-keys", routing::post(auth::create_api_key))
        .route("/api/auth/api-keys/{id}", routing::delete(auth::revoke_api_key))
        .route("/api/clips/upload", routing::post(clips::upload_clip))
        .route("/api/clips", routing::get(clips::list_clips))
        .route("/api/clips/{id}", routing::get(clips::get_clip))
        .route("/api/clips/{id}", routing::patch(clips::update_clip))
        .route("/api/clips/{id}", routing::delete(clips::delete_clip))
        .route("/api/clips/{id}/regenerate-share", routing::post(clips::regenerate_share))
        .route("/api/s/{share_id}/meta", routing::get(public::share_meta))
        .route("/api/media/{*path}", routing::get(public::serve_media))
        .route("/api/admin/users", routing::get(admin::list_users))
        .route("/api/admin/users/{id}", routing::get(admin::get_user))
        .route("/api/admin/users/{id}", routing::patch(admin::update_user))
        .route("/api/admin/users/{id}", routing::delete(admin::delete_user))
        .route("/api/admin/stats", routing::get(admin::get_stats))
        .route("/api/admin/clips", routing::get(admin::list_all_clips))
        .route("/api/admin/clips/{id}", routing::delete(admin::admin_delete_clip))
        .route("/api/admin/logs", routing::get(admin::get_logs))
}
