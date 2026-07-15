mod api;
mod auth;
mod config;
mod db;
mod email;
mod errors;
mod middleware;
mod storage;
mod thumbnail;

use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Request},
    http::{header, HeaderValue, Method, StatusCode},
    middleware as axum_middleware,
    response::IntoResponse,
    routing::get,
    Router,
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing_subscriber::EnvFilter;

pub struct AppState {
    pub pool: PgPool,
    pub config: config::Config,
    pub storage: storage::local::LocalStorage,
    pub rate_limiter: Arc<middleware::rate_limit::RateLimiter>,
    pub desktop_code_cache: api::auth::DesktopCodeCache,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            config: self.config.clone(),
            storage: self.storage.clone(),
            rate_limiter: Arc::clone(&self.rate_limiter),
            desktop_code_cache: self.desktop_code_cache.clone(),
        }
    }
}

impl axum::extract::FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl axum::extract::FromRef<AppState> for config::Config {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl axum::extract::FromRef<AppState> for storage::local::LocalStorage {
    fn from_ref(state: &AppState) -> Self {
        storage::local::LocalStorage::new(&state.config.storage_path)
    }
}

impl axum::extract::FromRef<AppState> for api::auth::DesktopCodeCache {
    fn from_ref(state: &AppState) -> Self {
        state.desktop_code_cache.clone()
    }
}

fn validate_env() {
    // Docker Compose passes empty strings for undefined vars rather than leaving
    // them unset, so env::var().is_err() is not sufficient — we must also check
    // that the value is non-empty.
    let required = [
        "DATABASE_URL",
        "JWT_SECRET",
    ];
    let mut ok = true;
    for name in &required {
        match std::env::var(name) {
            Ok(v) if !v.trim().is_empty() => {}
            _ => {
                eprintln!("FATAL: environment variable {name} is not set or is empty");
                ok = false;
            }
        }
    }
    if !ok {
        eprintln!("FATAL: one or more required environment variables are missing. See above.");
        std::process::exit(1);
    }
}

#[tokio::main]
async fn main() {
    // Validate required env vars BEFORE anything else, so failures are always
    // visible in Docker logs — even if tracing or the panic hook is not yet set up.
    validate_env();

    // Log panics before abort so they're visible in Docker logs
    std::panic::set_hook(Box::new(|info| {
        eprintln!("=== PANIC ===");
        eprintln!("{info}");
        let backtrace = backtrace::Backtrace::new();
        eprintln!("{:?}", backtrace);
        eprintln!("=============");
    }));

    if let Err(e) = dotenvy::dotenv() {
        tracing::warn!("Failed to load .env file: {e}");
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = config::Config::from_env();
    tracing::info!("Starting Prism Server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!(
        google_configured = %(!config.google_client_id.is_empty() && !config.google_client_secret.is_empty()),
        smtp_configured = %!config.smtp_host.is_empty(),
        jwt_secret_len = config.jwt_secret.len(),
        "config loaded",
    );

    let pool = db::init_pool(&config.database_url)
        .await
        .expect("Failed to initialize database pool");
    tracing::info!("Database connected and migrations applied");

    let storage = storage::local::LocalStorage::new(&config.storage_path);
    {
        let clip_dir = std::path::Path::new(&config.storage_path).join("clips");
        let thumb_dir = std::path::Path::new(&config.storage_path).join("thumbs");
        if let Err(e) = tokio::fs::create_dir_all(&clip_dir).await {
            tracing::warn!("Failed to create clip directory {}: {e}", clip_dir.display());
        }
        if let Err(e) = tokio::fs::create_dir_all(&thumb_dir).await {
            tracing::warn!("Failed to create thumb directory {}: {e}", thumb_dir.display());
        }
    }

    let site_origin: HeaderValue = config.site_url.parse().expect("Invalid SITE_URL");
    let rate_limiter = Arc::new(middleware::rate_limit::RateLimiter::new(config.rate_limit_per_min));

    let state = AppState {
        pool,
        config,
        storage,
        rate_limiter,
        desktop_code_cache: api::auth::new_desktop_code_cache(),
    };
    let mut allowed_origins = vec![
        "http://localhost:3000".parse::<HeaderValue>().unwrap(),
        "http://127.0.0.1:3000".parse::<HeaderValue>().unwrap(),
        "http://localhost:1420".parse::<HeaderValue>().unwrap(),
    ];
    if !allowed_origins.contains(&site_origin) {
        allowed_origins.push(site_origin);
    }
    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::COOKIE])
        .allow_credentials(true);

    let app = api::add_api_routes(Router::<AppState>::new())
        .route("/s/{share_id}", get(api::public::serve_share_page))
        .route("/u/{username}", get(api::public::serve_profile_page))
        .layer(axum_middleware::from_fn(middleware::timeout::timeout_middleware))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; media-src 'self' data: https:; connect-src 'self' https://accounts.google.com https://oauth2.googleapis.com https://www.googleapis.com",
            ),
        ))
        .layer(cors)
        .with_state(state);

    let addr: std::net::SocketAddr = format!(
        "{}:{}",
        std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
        std::env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8080".into())
            .parse::<u16>()
            .unwrap_or(8080),
    )
    .parse()
    .expect("Invalid server address");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("FATAL: Failed to bind to {addr}: {e}");
            std::process::exit(1);
        });

    tracing::info!("Listening on http://{}", listener.local_addr().unwrap_or(addr));
    if let Err(e) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    {
        eprintln!("FATAL: Server error: {e}");
        std::process::exit(1);
    }
}

async fn rate_limit_middleware(
    axum::extract::State(state): axum::extract::State<AppState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    request: Request,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    let path = request.uri().path();
    let is_auth_endpoint = path.starts_with("/api/auth/login")
        || path.starts_with("/api/auth/register")
        || path.starts_with("/api/auth/2fa/")
        || path.starts_with("/api/auth/verify-code")
        || path.starts_with("/api/auth/resend-verification")
        || path.starts_with("/api/auth/change-password");

    let ip_key = format!("ip:{}", addr);

    if is_auth_endpoint {
        // Stricter rate limit for auth endpoints: 10 req/min per IP
        if !state.rate_limiter.check_auth(&ip_key) {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                axum::Json(serde_json::json!({"error": "Too many requests. Please slow down."})),
            )
                .into_response();
        }
    } else if !state.rate_limiter.check(&ip_key) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            axum::Json(serde_json::json!({"error": "Rate limit exceeded"})),
        )
            .into_response();
    }

    next.run(request).await
}
