// Standard library
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

// 3rd party crates
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

// Project imports
use crate::utility::rate_limiter::traits::RateLimiter;
use crate::utility::rate_limiter::types::{RateLimitConfig, TokenBucketRateLimiter};

use super::constants::IP_SERVICES;
use super::functions::{
    default_max_requests_per_hour, default_min_consensus, default_network_retry_interval,
};
use super::types::{IpDetection, IpDetector, IpService, IpVersion};

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
        let rate_limiters = IP_SERVICES
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
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .user_agent("fariba-ddns/1.0")
                .build()
                .unwrap_or_default(),
        }
    }

    /// Detects the current public IP address with consensus validation
    pub async fn detect_ip(&self, ip_version: IpVersion) -> Option<IpAddr> {
        let mut responses = Vec::new();
        let mut errors = 0;

        for (idx, service) in IP_SERVICES.iter().enumerate() {
            // Skip IPv6 check for services that don't support it
            if matches!(ip_version, IpVersion::V6) && !service.supports_v6 {
                continue;
            }

            // Check rate limit
            if !self.rate_limiters[idx].acquire().await {
                debug!("Rate limit reached for service: {}", service.base_url);
                continue;
            }

            match self.query_ip_service(service, ip_version).await {
                Ok(ip) => {
                    debug!(
                        "Successfully got IP {} from service {}",
                        ip, service.base_url
                    );
                    responses.push(ip);
                }
                Err(e) => {
                    error!("Failed to query IP service {}: {}", service.base_url, e);
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
        for (idx, service) in IP_SERVICES.iter().enumerate() {
            if !self.rate_limiters[idx].acquire().await {
                continue;
            }

            match self.client.get(service.base_url).send().await {
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
        service: &IpService,
        ip_version: IpVersion,
    ) -> Result<IpAddr, Box<dyn std::error::Error>> {
        let path = match ip_version {
            IpVersion::V4 => service.v4_path,
            IpVersion::V6 => service.v6_path,
        };

        let url = format!("{}{}", service.base_url, path);
        let response = self.client.get(&url).send().await?.text().await?;

        // Try to parse as JSON first (for services that return JSON)
        if response.trim().starts_with('{') {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response) {
                // Try common JSON fields for IP addresses
                for field in ["ip", "address", "ipAddress", "query"] {
                    if let Some(ip_str) = json.get(field).and_then(|v| v.as_str()) {
                        if let Ok(ip) = ip_str.parse() {
                            return self.validate_ip_version(ip, ip_version);
                        }
                    }
                }
            }
        }

        // Try direct parsing if not JSON or JSON parsing failed
        let ip: IpAddr = response.trim().parse()?;
        self.validate_ip_version(ip, ip_version)
    }

    fn validate_ip_version(
        &self,
        ip: IpAddr,
        expected_version: IpVersion,
    ) -> Result<IpAddr, Box<dyn std::error::Error>> {
        match (ip, expected_version) {
            (IpAddr::V4(_), IpVersion::V4) | (IpAddr::V6(_), IpVersion::V6) => Ok(ip),
            _ => Err("IP version mismatch".into()),
        }
    }
}
