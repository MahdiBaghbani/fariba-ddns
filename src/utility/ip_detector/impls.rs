// Standard library
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::{Duration, Instant};

// 3rd party crates
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

// Project imports
use crate::utility::rate_limiter::traits::RateLimiter;
use crate::utility::rate_limiter::types::{RateLimitConfig, TokenBucketRateLimiter};

use super::constants::IP_CHECK_SERVICES;
use super::functions::{
    default_max_requests_per_hour, default_min_consensus, default_network_retry_interval,
};
use super::types::{IpDetection, IpDetector, IpVersion};

impl Default for IpDetection {
    fn default() -> Self {
        Self {
            max_requests_per_hour: default_max_requests_per_hour(),
            min_consensus: default_min_consensus(),
            network_retry_interval: default_network_retry_interval(),
        }
    }
}

impl IpDetector {
    pub fn new(config: IpDetection) -> Self {
        let rate_limiters = IP_CHECK_SERVICES
            .iter()
            .map(|_| {
                Arc::new(TokenBucketRateLimiter::new(RateLimitConfig {
                    max_requests: config.max_requests_per_hour,
                    window_secs: 3600, // 1 hour
                })) as Arc<dyn RateLimiter>
            })
            .collect();

        Self {
            config,
            rate_limiters,
            last_check: Arc::new(RwLock::new(Instant::now())),
            client: reqwest::Client::new(),
        }
    }

    /// Detects the current public IP address with consensus validation
    pub async fn detect_ip(&self, ip_version: IpVersion) -> Option<IpAddr> {
        let mut responses = Vec::new();
        let mut errors = 0;

        for (idx, service) in IP_CHECK_SERVICES.iter().enumerate() {
            // Check rate limit
            if !self.rate_limiters[idx].acquire().await {
                debug!("Rate limit reached for service: {}", service);
                continue;
            }

            match self.query_ip_service(service, ip_version).await {
                Ok(ip) => {
                    responses.push(ip);
                }
                Err(e) => {
                    error!("Failed to query IP service {}: {}", service, e);
                    errors += 1;
                }
            }

            self.rate_limiters[idx].release().await;
        }

        // Check if we have enough responses for consensus
        if responses.len() < self.config.min_consensus as usize {
            warn!(
                "Insufficient consensus: got {} responses, need {}",
                responses.len(),
                self.config.min_consensus
            );
            return None;
        }

        // Find the most common IP address
        let mut ip_counts = std::collections::HashMap::new();
        for ip in &responses {
            *ip_counts.entry(*ip).or_insert(0) += 1;
        }

        ip_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(ip, count)| {
                if count >= self.config.min_consensus {
                    Some(ip)
                } else {
                    None
                }
            })
            .flatten()
    }

    /// Checks network connectivity
    pub async fn check_network(&self) -> bool {
        for (idx, service) in IP_CHECK_SERVICES.iter().enumerate() {
            if !self.rate_limiters[idx].acquire().await {
                continue;
            }

            match self
                .client
                .get(*service)
                .timeout(Duration::from_secs(5))
                .send()
                .await
            {
                Ok(_) => {
                    self.rate_limiters[idx].release().await;
                    return true;
                }
                Err(_) => {
                    self.rate_limiters[idx].release().await;
                    continue;
                }
            }
        }
        false
    }

    /// Gets the network retry interval from the configuration
    pub fn get_network_retry_interval(&self) -> u64 {
        self.config.network_retry_interval
    }

    async fn query_ip_service(
        &self,
        service: &str,
        ip_version: IpVersion,
    ) -> Result<IpAddr, Box<dyn std::error::Error>> {
        let url = match ip_version {
            IpVersion::V4 => format!("{}", service),
            IpVersion::V6 => format!("{}/ipv6", service),
        };

        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .text()
            .await?;

        let ip: IpAddr = response.trim().parse()?;

        // Validate IP version matches what we requested
        match (ip, ip_version) {
            (IpAddr::V4(_), IpVersion::V4) | (IpAddr::V6(_), IpVersion::V6) => Ok(ip),
            _ => Err("IP version mismatch".into()),
        }
    }
}
