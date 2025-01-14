// Standard library
use std::sync::Arc;

// 3rd party crates
use serde::Deserialize;
use tokio::sync::Semaphore;
use tokio::time::{Duration, Instant};

/// Rate limiting configuration for DNS providers
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum number of requests per time window
    pub max_requests: u32,
    /// Time window in seconds
    pub window_secs: u64,
}

/// A token bucket rate limiter implementation
pub struct TokenBucketRateLimiter {
    pub semaphore: Arc<Semaphore>,
    pub window: Duration,
    pub last_refill: tokio::sync::Mutex<Instant>,
}
