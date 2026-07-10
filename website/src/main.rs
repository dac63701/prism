mod api;
mod auth;
mod config;
mod db;
mod errors;
mod frontend;
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
    pub frontend: frontend::FrontendStatic,
    pub rate_limiter: middleware::rate_limit::RateLimiter,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            config: self.config.clone(),
            storage: self.storage.clone(),
            frontend: frontend::FrontendStatic::new(&self.config.storage_path),
            rate_limiter: middleware::rate_limit::RateLimiter::new(self.config.rate_limit_per_min),
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
        // HACK: storage doesn't support Clone directly; rebuild from config path
        storage::local::LocalStorage::new(&state.config.storage_path)
    }
}

impl axum::extract::FromRef<AppState> for Arc<frontend::FrontendStatic> {
    fn from_ref(state: &AppState) -> Self {
        Arc::new(frontend::FrontendStatic::new(&state.config.storage_path))
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = config::Config::from_env();
    tracing::info!("Starting Prism Server v{}", env!("CARGO_PKG_VERSION"));

    let pool = db::init_pool(&config.database_url)
        .await
        .expect("Failed to initialize database pool");
    tracing::info!("Database connected and migrations applied");

    let storage = storage::local::LocalStorage::new(&config.storage_path);
    {
        let clip_dir = std::path::Path::new(&config.storage_path).join("clips");
        let thumb_dir = std::path::Path::new(&config.storage_path).join("thumbs");
        let _ = tokio::fs::create_dir_all(&clip_dir).await;
        let _ = tokio::fs::create_dir_all(&thumb_dir).await;
    }

    let frontend_dir = "frontend/dist";
    tracing::info!(
        "Frontend directory: {} (exists: {})",
        frontend_dir,
        std::path::Path::new(frontend_dir).exists()
    );
    let frontend = frontend::FrontendStatic::new(frontend_dir);
    let rate_limiter = middleware::rate_limit::RateLimiter::new(config.rate_limit_per_min);

    let state = AppState {
        pool,
        config,
        storage,
        frontend,
        rate_limiter,
    };

    let cors = CorsLayer::new()
        .allow_origin([
            "https://goprism.studio".parse::<HeaderValue>().unwrap(),
            "http://localhost:5173".parse::<HeaderValue>().unwrap(),
            "http://localhost:1420".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_credentials(true);

    let app = api::add_api_routes(Router::<AppState>::new())
        .route("/s/{share_id}", get(api::public::serve_share_page))
        .fallback(handle_frontend)
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
                "default-src 'self'; \
                 script-src 'self'; \
                 style-src 'self' 'unsafe-inline'; \
                 img-src 'self' data:; \
                 media-src 'self'; \
                 connect-src 'self' https://goprism.studio; \
                 frame-ancestors 'none'",
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

    tracing::info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn handle_frontend(
    axum::extract::State(state): axum::extract::State<AppState>,
    request: Request,
) -> impl axum::response::IntoResponse {
    state.frontend.serve(request).await
}

async fn rate_limit_middleware(
    axum::extract::State(state): axum::extract::State<AppState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    request: Request,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    let key = format!("ip:{}", addr);
    if !state.rate_limiter.check(&key) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            axum::Json(serde_json::json!({"error": "Rate limit exceeded"})),
        )
            .into_response();
    }
    next.run(request).await
}
