use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    capacity: f64,
    refill_rate: f64,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            last_refill: Instant::now(),
            capacity,
            refill_rate,
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
            self.last_refill = now;
        }
    }

    fn consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

pub struct RateLimiter {
    buckets: Mutex<HashMap<String, TokenBucket>>,
    capacity: f64,
    auth_capacity: f64,
}

impl RateLimiter {
    pub fn new(requests_per_min: u64) -> Self {
        let capacity = requests_per_min.max(1) as f64;
        // Auth endpoints get a stricter cap: 10 requests per minute per key
        let auth_capacity = 10.0;
        Self {
            buckets: Mutex::new(HashMap::new()),
            capacity,
            auth_capacity,
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.capacity, self.capacity / 60.0));
        bucket.consume()
    }

    /// Check against an auth-scoped rate limit (stricter: 10 req/min per key).
    /// Returns true if allowed, false if rate limited.
    pub fn check_auth(&self, key: &str) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets.entry(format!("auth:{key}")).or_insert_with(|| {
            TokenBucket::new(self.auth_capacity, self.auth_capacity / 60.0)
        });
        bucket.consume()
    }

    pub fn cleanup(&self) {
        let mut buckets = self.buckets.lock().unwrap();
        buckets.retain(|_, b| {
            b.refill();
            b.tokens < b.capacity * 2.0
        });
    }
}
