use std::env;

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
    pub site_url: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub desktop_scheme_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        if jwt_secret.len() < 32 {
            eprintln!("WARNING: JWT_SECRET is shorter than 32 chars — use a strong secret");
        }

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            jwt_secret,
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            server_port: env::var("SERVER_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            storage_path: env::var("STORAGE_PATH").unwrap_or_else(|_| "./data".into()),
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
        }
    }

    pub fn public_url(&self) -> String {
        self.site_url.clone()
    }
}
