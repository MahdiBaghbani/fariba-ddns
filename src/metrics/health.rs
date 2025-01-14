// Standard library
use std::sync::Arc;
use std::time::{Duration, Instant};

// 3rd party crates
use tokio::sync::RwLock;

/// Health status of the service
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Whether the service is healthy
    pub healthy: bool,
    /// Last successful operation time
    pub last_success: Option<Instant>,
    /// Last failure time
    pub last_failure: Option<Instant>,
    /// Current error message, if any
    pub error: Option<String>,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            healthy: true,
            last_success: None,
            last_failure: None,
            error: None,
            consecutive_failures: 0,
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Maximum allowed consecutive failures
    pub max_consecutive_failures: u32,
    /// Maximum time without successful operation
    pub max_time_without_success: Duration,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            max_consecutive_failures: 3,
            max_time_without_success: Duration::from_secs(900), // 15 minutes
        }
    }
}

/// Health checker for monitoring service health
#[derive(Debug)]
pub struct HealthChecker {
    status: Arc<RwLock<HealthStatus>>,
    config: HealthConfig,
}

impl HealthChecker {
    /// Creates a new HealthChecker with default configuration
    pub fn new() -> Self {
        Self::with_config(HealthConfig::default())
    }

    /// Creates a new HealthChecker with custom configuration
    pub fn with_config(config: HealthConfig) -> Self {
        Self {
            status: Arc::new(RwLock::new(HealthStatus::default())),
            config,
        }
    }

    /// Records a successful operation
    pub async fn record_success(&self) {
        let mut status = self.status.write().await;
        status.healthy = true;
        status.last_success = Some(Instant::now());
        status.consecutive_failures = 0;
        status.error = None;
    }

    /// Records a failed operation
    pub async fn record_failure(&self, error: String) {
        let mut status = self.status.write().await;
        status.last_failure = Some(Instant::now());
        status.consecutive_failures += 1;
        status.error = Some(error);

        // Check if we've exceeded the failure threshold
        if status.consecutive_failures >= self.config.max_consecutive_failures {
            status.healthy = false;
        }

        // Check if we've exceeded the time without success threshold
        if let Some(last_success) = status.last_success {
            if last_success.elapsed() > self.config.max_time_without_success {
                status.healthy = false;
            }
        }
    }

    /// Gets the current health status
    pub async fn get_status(&self) -> HealthStatus {
        let status = self.status.read().await;
        status.clone()
    }

    /// Checks if the service is currently healthy
    pub async fn is_healthy(&self) -> bool {
        self.status.read().await.healthy
    }
}
