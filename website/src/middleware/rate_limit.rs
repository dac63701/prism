use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Instant;

use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthUser;

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    capacity: f64,
    rate: f64,
}

impl TokenBucket {
    fn new(capacity: f64, rate: f64) -> Self {
        Self {
            tokens: capacity,
            last_refill: Instant::now(),
            capacity,
            rate,
        }
    }

    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate).min(self.capacity);
        self.last_refill = now;
    }
}

pub struct RateLimiter {
    buckets: Mutex<HashMap<String, TokenBucket>>,
    capacity: f64,
    rate: f64,
}

impl RateLimiter {
    pub fn new(requests_per_min: u64) -> Self {
        let rate = requests_per_min as f64 / 60.0;
        Self {
            buckets: Mutex::new(HashMap::new()),
            capacity: requests_per_min as f64,
            rate,
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.capacity, self.rate));
        bucket.try_consume()
    }

    pub fn cleanup(&self) {
        let mut buckets = self.buckets.lock().unwrap();
        buckets.retain(|_, b| {
            b.refill();
            b.tokens < b.capacity * 2.0
        });
    }
}

pub async fn rate_limit_middleware(
    auth: Option<AuthUser>,
    ConnectInfo(addr): ConnectInfo<IpAddr>,
    limiter: axum::extract::State<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    let key = auth
        .as_ref()
        .map(|u| format!("user:{}", u.user_id))
        .unwrap_or_else(|| format!("ip:{}", addr));

    if !limiter.check(&key) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error": "Rate limit exceeded. Try again later."})),
        )
            .into_response();
    }

    next.run(request).await
}
