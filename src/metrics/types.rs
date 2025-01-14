// Standard library
use std::sync::Arc;
use std::time::Instant;

// 3rd party crates
use tokio::sync::RwLock;

/// Metrics for DNS operations
#[derive(Debug, Default, Clone)]
pub struct DnsMetrics {
    /// Total number of DNS record updates attempted
    pub update_attempts: u64,
    /// Number of successful DNS record updates
    pub update_successes: u64,
    /// Number of failed DNS record updates
    pub update_failures: u64,
    /// Number of DNS records that were already up to date
    pub already_up_to_date: u64,
    /// Number of rate limit hits
    pub rate_limit_hits: u64,
    /// Number of API timeouts
    pub timeouts: u64,
    /// Last successful update time
    pub last_success: Option<Instant>,
    /// Last failure time
    pub last_failure: Option<Instant>,
    /// IPv4 update metrics
    pub ipv4: IpVersionMetrics,
    /// IPv6 update metrics
    pub ipv6: IpVersionMetrics,
}

/// Metrics specific to IP version updates
#[derive(Debug, Default, Clone)]
pub struct IpVersionMetrics {
    /// Number of successful updates
    pub successes: u64,
    /// Number of failed updates
    pub failures: u64,
    /// Last known IP address
    pub last_ip: Option<String>,
}

/// Thread-safe metrics manager
#[derive(Debug, Default)]
pub struct MetricsManager {
    metrics: Arc<RwLock<DnsMetrics>>,
}

impl MetricsManager {
    /// Creates a new MetricsManager
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(DnsMetrics::default())),
        }
    }

    /// Records a successful DNS update
    pub async fn record_success(&self, is_ipv6: bool, ip: String) {
        let mut metrics = self.metrics.write().await;
        metrics.update_attempts += 1;
        metrics.update_successes += 1;
        metrics.last_success = Some(Instant::now());

        let ip_metrics = if is_ipv6 {
            &mut metrics.ipv6
        } else {
            &mut metrics.ipv4
        };
        ip_metrics.successes += 1;
        ip_metrics.last_ip = Some(ip);
    }

    /// Records a failed DNS update
    pub async fn record_failure(&self, is_ipv6: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.update_attempts += 1;
        metrics.update_failures += 1;
        metrics.last_failure = Some(Instant::now());

        let ip_metrics = if is_ipv6 {
            &mut metrics.ipv6
        } else {
            &mut metrics.ipv4
        };
        ip_metrics.failures += 1;
    }

    /// Records a rate limit hit
    pub async fn record_rate_limit(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.rate_limit_hits += 1;
    }

    /// Records a timeout
    pub async fn record_timeout(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.timeouts += 1;
    }

    /// Records that a DNS record was already up to date
    pub async fn record_already_up_to_date(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.already_up_to_date += 1;
    }

    /// Gets a snapshot of the current metrics
    pub async fn get_snapshot(&self) -> DnsMetrics {
        (*self.metrics.read().await).clone()
    }
}
