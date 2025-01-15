// Standard library
use std::fmt;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;

// 3rd party crates
use async_trait::async_trait;

// Project modules
use crate::providers::traits::DnsProvider;
use crate::utility::rate_limiter::traits::RateLimiter;
use crate::utility::rate_limiter::types::TokenBucketRateLimiter;

// Current module imports
use super::errors::{CloudflareError, CloudflareValidationError};
use super::functions::{create_reqwest_client, update_dns_records};
use super::types::{CfConfig, Cloudflare, IpVersion};

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

impl CfConfig {
    pub fn validate(&self) -> Result<(), CloudflareValidationError> {
        if self.zone_id.trim().is_empty() {
            return Err(CloudflareValidationError::MissingZoneId);
        }

        if self.api_token.trim().is_empty() {
            return Err(CloudflareValidationError::MissingApiToken);
        }

        if self.name.trim().is_empty() {
            return Err(CloudflareValidationError::MissingName);
        }

        if self.subdomains.is_empty() {
            return Err(CloudflareValidationError::NoSubdomains);
        }

        // Validate rate limit configuration
        if self.rate_limit.max_requests == 0 {
            return Err(CloudflareValidationError::InvalidRateLimit(
                "max_requests must be greater than 0".into(),
            ));
        }

        if self.rate_limit.window_secs == 0 {
            return Err(CloudflareValidationError::InvalidRateLimit(
                "window_secs must be greater than 0".into(),
            ));
        }

        // Validate subdomain configurations
        let mut has_ipv4 = false;
        let mut has_ipv6 = false;
        for subdomain in &self.subdomains {
            match subdomain.ip_version {
                super::types::IpVersion::V4 => has_ipv4 = true,
                super::types::IpVersion::V6 => has_ipv6 = true,
                super::types::IpVersion::Both => {
                    has_ipv4 = true;
                    has_ipv6 = true;
                }
            }
        }

        // Ensure at least one IP version is enabled
        if !has_ipv4 && !has_ipv6 {
            return Err(CloudflareValidationError::InvalidIpVersion(
                "At least one subdomain must have IPv4 or IPv6 enabled".into(),
            ));
        }

        Ok(())
    }
}

impl Default for IpVersion {
    fn default() -> Self {
        Self::Both
    }
}

#[async_trait]
impl DnsProvider for Cloudflare {
    type Config = CfConfig;
    type Error = CloudflareError;

    fn new(config: Self::Config) -> Result<Self, Self::Error> {
        Self::new(config)
    }

    async fn update_dns_records_v4(&self, ip: &Ipv4Addr) -> Result<(), Self::Error> {
        update_dns_records(self, &IpAddr::V4(*ip)).await
    }

    async fn update_dns_records_v6(&self, ip: &Ipv6Addr) -> Result<(), Self::Error> {
        // Check if any subdomain needs IPv6
        let needs_ipv6 = self
            .config
            .subdomains
            .iter()
            .any(|subdomain| matches!(subdomain.ip_version, IpVersion::V6 | IpVersion::Both));

        if !needs_ipv6 {
            return Ok(());
        }
        update_dns_records(self, &IpAddr::V6(*ip)).await
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
