// 3rd party crates
use async_trait::async_trait;

/// Rate limiter trait for implementing different rate limiting strategies
#[async_trait]
pub trait RateLimiter: Send + Sync {
    /// Acquire permission to make a request
    async fn acquire(&self) -> bool;
    /// Release a request slot
    async fn release(&self);
}
