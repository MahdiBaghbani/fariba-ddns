// Standard library
use std::fmt;
use std::future::Future;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::Arc;

// 3rd party crates
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

// Project modules
use crate::metrics::{HealthChecker, MetricsManager};
use crate::providers::traits::DnsProvider;
use crate::utility::rate_limiter::traits::RateLimiter;
use crate::utility::rate_limiter::types::{RateLimitConfig, TokenBucketRateLimiter};
use crate::utility::SharedDnsCache;

use super::errors::CloudflareError;
use super::functions::create_reqwest_client;

/// Represents a client for interacting with the Cloudflare API.
pub struct Cloudflare {
    pub config: CfConfig,
    pub client: Client,
    rate_limiter: Arc<dyn RateLimiter>,
    metrics: Arc<MetricsManager>,
    health: Arc<HealthChecker>,
    pub cache: SharedDnsCache,
}

// Manual Debug implementation for Cloudflare
impl fmt::Debug for Cloudflare {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cloudflare")
            .field("config", &self.config)
            .field("client", &self.client)
            .field("rate_limiter", &"<rate limiter>")
            .field("metrics", &self.metrics)
            .field("health", &self.health)
            .field("cache", &self.cache)
            .finish()
    }
}

// Manual Clone implementation for Cloudflare
impl Clone for Cloudflare {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            client: self.client.clone(),
            rate_limiter: Arc::clone(&self.rate_limiter),
            metrics: Arc::clone(&self.metrics),
            health: Arc::clone(&self.health),
            cache: self.cache.clone(),
        }
    }
}

/// Configuration for Cloudflare API interactions.
#[derive(Debug, Deserialize, Clone)]
pub struct CfConfig {
    pub enabled: bool,
    pub name: String,
    pub zone_id: String,
    pub api_token: String,
    #[serde(default)]
    pub enable_ipv6: bool,
    #[serde(default = "default_rate_limit_config")]
    pub rate_limit: RateLimitConfig,
    pub subdomains: Vec<CfSubDomain>,
}

fn default_rate_limit_config() -> RateLimitConfig {
    RateLimitConfig {
        max_requests: 30, // Cloudflare's default rate limit is 1200/5min
        window_secs: 60,  // 1-minute window
    }
}

/// Represents a subdomain configuration in Cloudflare.
/// An empty name represents the root domain.
#[derive(Debug, Deserialize, Clone)]
pub struct CfSubDomain {
    #[serde(default)]
    pub name: String,
}

/// Represents the response from a DNS record request.
#[derive(Debug, Deserialize)]
pub struct DnsResponse {
    pub result: Vec<DnsResponseResult>,
}

/// Details of the DNS response result.
#[derive(Debug, Deserialize)]
pub struct DnsResponseResult {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub r#type: String,
}

/// Represents the response from a zone request.
#[derive(Debug, Deserialize)]
pub struct ZoneResponse {
    pub result: ZoneResponseResult,
    pub success: bool,
}

/// Details of the zone response result.
#[derive(Debug, Deserialize)]
pub struct ZoneResponseResult {
    pub status: String,
}

impl Cloudflare {
    /// Creates a new Cloudflare instance with the provided metrics and health monitoring
    pub fn new_with_monitoring(
        config: CfConfig,
        metrics: Arc<MetricsManager>,
        health: Arc<HealthChecker>,
        cache: SharedDnsCache,
    ) -> Result<Self, CloudflareError> {
        let client = create_reqwest_client(&config)?;
        let rate_limiter = Arc::new(TokenBucketRateLimiter::new(config.rate_limit.clone()));

        Ok(Self {
            config,
            client,
            rate_limiter,
            metrics,
            health,
            cache,
        })
    }

    /// Acquires a rate limit permit before making an API call
    pub async fn with_rate_limit<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: Future<Output = Result<T, E>>,
        E: From<CloudflareError>,
    {
        if !self.rate_limiter.acquire().await {
            self.metrics.record_rate_limit().await;
            return Err(CloudflareError::RateLimited(self.config.name.clone()).into());
        }

        let result = f.await;
        self.rate_limiter.release().await;
        result
    }
}

#[async_trait]
impl DnsProvider for Cloudflare {
    type Config = CfConfig;
    type Error = CloudflareError;

    fn new(config: Self::Config) -> Result<Self, Self::Error> {
        // This is just a convenience method that creates default monitoring
        // It should only be used for testing or when monitoring is not required
        Self::new_with_monitoring(
            config,
            Arc::new(MetricsManager::new()),
            Arc::new(HealthChecker::new()),
            SharedDnsCache::new(60), // Default 1 minute TTL
        )
    }

    async fn update_dns_records(&self, ip: &Ipv4Addr) -> Result<(), Self::Error> {
        use super::functions::update_dns_records;
        let result = update_dns_records(self, ip).await;
        match &result {
            Ok(_) => {
                self.metrics.record_success(false, ip.to_string()).await;
                self.health.record_success().await;
            }
            Err(e) => {
                self.metrics.record_failure(false).await;
                self.health.record_failure(e.to_string()).await;
            }
        }
        result
    }

    async fn update_dns_records_v6(&self, ip: &Ipv6Addr) -> Result<(), Self::Error> {
        if !self.config.enable_ipv6 {
            return Ok(());
        }
        use super::functions::update_dns_records_v6;
        let result = update_dns_records_v6(self, ip).await;
        match &result {
            Ok(_) => {
                self.metrics.record_success(true, ip.to_string()).await;
                self.health.record_success().await;
            }
            Err(e) => {
                self.metrics.record_failure(true).await;
                self.health.record_failure(e.to_string()).await;
            }
        }
        result
    }

    fn validate_config(&self) -> Result<(), Self::Error> {
        // Basic validation
        if self.config.api_token.is_empty() || self.config.api_token == "your_api_token_here" {
            return Err(CloudflareError::InvalidApiToken(self.config.name.clone()));
        }
        if self.config.zone_id.is_empty() {
            return Err(CloudflareError::InvalidZoneId(self.config.name.clone()));
        }
        if self.config.subdomains.is_empty() {
            return Err(CloudflareError::NoSubdomains(self.config.name.clone()));
        }

        // Rate limit validation
        if self.config.rate_limit.max_requests == 0 {
            return Err(CloudflareError::InvalidRateLimit {
                zone: self.config.name.clone(),
                reason: "max_requests must be greater than 0".to_string(),
            });
        }
        if self.config.rate_limit.window_secs == 0 {
            return Err(CloudflareError::InvalidRateLimit {
                zone: self.config.name.clone(),
                reason: "window_secs must be greater than 0".to_string(),
            });
        }

        // Subdomain validation
        for subdomain in &self.config.subdomains {
            if subdomain.name.is_empty() {
                return Err(CloudflareError::InvalidSubdomain {
                    zone: self.config.name.clone(),
                    subdomain: subdomain.name.clone(),
                });
            }
            // Check for valid domain name format
            if !is_valid_domain_name(&subdomain.name) {
                return Err(CloudflareError::InvalidSubdomain {
                    zone: self.config.name.clone(),
                    subdomain: subdomain.name.clone(),
                });
            }
        }

        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    fn get_name(&self) -> &str {
        &self.config.name
    }
}

/// Validates a domain name according to RFC 1035
fn is_valid_domain_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 253 {
        return false;
    }

    let labels: Vec<&str> = name.split('.').collect();
    if labels.len() < 2 {
        return false;
    }

    labels.iter().all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    })
}
