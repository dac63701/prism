use axum::{routing, Router};

use crate::AppState;

pub mod admin;
pub mod auth;
pub mod clips;
pub mod public;
pub mod tags;

pub fn add_api_routes(router: Router<AppState>) -> Router<AppState> {
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
        .route("/api/clips/upload", routing::post(clips::upload_clip))
        .route("/api/clips", routing::get(clips::list_clips))
        .route("/api/clips/{id}", routing::get(clips::get_clip))
        .route("/api/clips/{id}", routing::patch(clips::update_clip))
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
