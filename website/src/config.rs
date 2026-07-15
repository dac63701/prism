use std::env;

/// Return the value of `name` or exit with a fatally-visible error.
/// Docker Compose passes empty strings for undefined vars, so we treat
/// empty/missing identically — the caller in main.rs already validates
/// these vars exist, but this ensures any other call path also fails clearly.
fn require_env(name: &str) -> String {
    match env::var(name) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            eprintln!("FATAL: environment variable {name} is not set or is empty");
            std::process::exit(1);
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub server_host: String,
    pub server_port: u16,
    pub storage_path: String,
    pub max_upload_size_mb: u64,
    pub default_max_storage_gb: u64,
    pub rate_limit_per_min: u64,
    pub max_failed_login_attempts: i32,
    pub login_lockout_minutes: i32,
    pub site_url: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub desktop_scheme_url: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from_address: String,
    pub smtp_from_name: String,
}

impl Config {
    pub fn from_env() -> Self {
        let jwt_secret = require_env("JWT_SECRET");
        if jwt_secret.len() < 32 {
            eprintln!("WARNING: JWT_SECRET is shorter than 32 chars — use a strong secret");
        }

        Self {
            database_url: require_env("DATABASE_URL"),
            jwt_secret,
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            server_port: env::var("SERVER_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            max_failed_login_attempts: env::var("MAX_FAILED_LOGIN_ATTEMPTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            login_lockout_minutes: env::var("LOGIN_LOCKOUT_MINUTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(15),
            storage_path: env::var("STORAGE_PATH").unwrap_or_else(|_| "/data".into()),
            max_upload_size_mb: env::var("MAX_UPLOAD_SIZE_MB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500),
            default_max_storage_gb: env::var("DEFAULT_MAX_STORAGE_GB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            rate_limit_per_min: env::var("RATE_LIMIT_REQUESTS_PER_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            site_url: env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:3000".into()),
            google_client_id: env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            google_client_secret: env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
            google_redirect_uri: env::var("GOOGLE_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:8080/api/auth/google/callback".into()),
            desktop_scheme_url: env::var("DESKTOP_SCHEME_URL")
                .unwrap_or_else(|_| "prism://auth/callback".into()),
            smtp_host: env::var("SMTP_HOST").unwrap_or_default(),
            smtp_port: env::var("SMTP_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(587),
            smtp_username: env::var("SMTP_USERNAME").unwrap_or_default(),
            smtp_password: env::var("SMTP_PASSWORD").unwrap_or_default(),
            smtp_from_address: env::var("SMTP_FROM_ADDRESS").unwrap_or_else(|_| "noreply@prism.com".into()),
            smtp_from_name: env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "Prism".into()),
        }
    }

    pub fn public_url(&self) -> String {
        self.site_url.clone()
    }
}
