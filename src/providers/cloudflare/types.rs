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
use crate::providers::traits::DnsProvider;
use crate::utility::rate_limiter::traits::RateLimiter;
use crate::utility::rate_limiter::types::{RateLimitConfig, TokenBucketRateLimiter};

use super::errors::CloudflareError;
use super::functions::create_reqwest_client;

/// Represents a client for interacting with the Cloudflare API.
/// This client handles DNS record management operations including:
/// - Creating DNS records
/// - Updating DNS records
/// - Fetching DNS records
/// - Managing both IPv4 (A) and IPv6 (AAAA) records
///
/// The client includes built-in rate limiting to respect Cloudflare's API limits.
pub struct Cloudflare {
    pub config: CfConfig,
    pub client: Client,
    rate_limiter: Arc<dyn RateLimiter>,
}

// Manual Debug implementation for Cloudflare
impl fmt::Debug for Cloudflare {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cloudflare")
            .field("config", &self.config)
            .field("client", &self.client)
            .field("rate_limiter", &"<rate limiter>")
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
        }
    }
}

/// Configuration for Cloudflare API interactions.
/// This struct holds all necessary settings for connecting to and managing
/// DNS records through the Cloudflare API.
#[derive(Debug, Deserialize, Clone)]
pub struct CfConfig {
    /// Whether this Cloudflare configuration is enabled
    pub enabled: bool,
    /// The domain name (e.g., "example.com")
    pub name: String,
    /// The Cloudflare zone ID for the domain
    pub zone_id: String,
    /// The Cloudflare API token with appropriate permissions
    pub api_token: String,
    /// Whether to enable IPv6 (AAAA) record management
    #[serde(default)]
    pub enable_ipv6: bool,
    /// Rate limiting configuration to respect Cloudflare's API limits
    #[serde(default = "default_rate_limit_config")]
    pub rate_limit: RateLimitConfig,
    /// List of subdomains to manage
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
    /// The subdomain name (e.g., "www" for www.example.com)
    /// Leave empty for root domain
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
    /// The record ID
    pub id: String,
    /// The record content (IP address)
    pub content: String,
    /// The record type (A or AAAA)
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
    /// The zone status (e.g., "active")
    pub status: String,
}

impl Cloudflare {
    /// Creates a new Cloudflare instance with the provided configuration.
    /// This will initialize the HTTP client and rate limiter.
    pub fn new(config: CfConfig) -> Result<Self, CloudflareError> {
        let client = create_reqwest_client(&config)?;
        let rate_limiter = Arc::new(TokenBucketRateLimiter::new(config.rate_limit.clone()));

        Ok(Self {
            config,
            client,
            rate_limiter,
        })
    }

    /// Acquires a rate limit permit before making an API call.
    /// This ensures we respect Cloudflare's API rate limits.
    pub async fn with_rate_limit<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: Future<Output = Result<T, E>>,
        E: From<CloudflareError>,
    {
        if !self.rate_limiter.acquire().await {
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
        Self::new(config)
    }

    async fn update_dns_records(&self, ip: &Ipv4Addr) -> Result<(), Self::Error> {
        use super::functions::update_dns_records;
        update_dns_records(self, ip).await
    }

    async fn update_dns_records_v6(&self, ip: &Ipv6Addr) -> Result<(), Self::Error> {
        if !self.config.enable_ipv6 {
            return Ok(());
        }
        use super::functions::update_dns_records_v6;
        update_dns_records_v6(self, ip).await
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

        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    fn get_name(&self) -> &str {
        &self.config.name
    }
}
