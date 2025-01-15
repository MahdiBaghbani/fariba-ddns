// Standard library
use std::sync::Arc;
use std::time::Duration;

// 3rd party crates
use async_trait::async_trait;
use tokio::sync::Semaphore;
use tokio::time::Instant;

use super::traits::RateLimiter;
use super::types::{RateLimitConfig, TokenBucketRateLimiter};

impl TokenBucketRateLimiter {
    /// Create a new token bucket rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.max_requests as usize)),
            window: Duration::from_secs(config.window_secs),
            last_refill: tokio::sync::Mutex::new(Instant::now()),
        }
    }

    /// Refill the token bucket if enough time has passed
    async fn try_refill(&self) {
        let mut last_refill = self.last_refill.lock().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill);

        if elapsed >= self.window {
            self.semaphore
                .add_permits(self.semaphore.available_permits());
            *last_refill = now;
        }
    }
}

#[async_trait]
impl RateLimiter for TokenBucketRateLimiter {
    async fn acquire(&self) -> bool {
        self.try_refill().await;
        self.semaphore.try_acquire().is_ok()
    }

    async fn release(&self) {
        self.semaphore.add_permits(1);
    }
}
