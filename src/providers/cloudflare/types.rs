// Standard library
use std::sync::Arc;

// 3rd party crates

use reqwest::Client;
use serde::Deserialize;

// Project modules
use crate::providers::traits::DnsProvider;
use crate::utility::rate_limiter::traits::RateLimiter;
use crate::utility::rate_limiter::types::RateLimitConfig;

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
    pub rate_limiter: Arc<dyn RateLimiter>,
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
    /// Which IP versions to use for this subdomain
    #[serde(default)]
    pub ip_version: IpVersion,
}

/// Specifies which IP versions should be used for a subdomain
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum IpVersion {
    /// Use only IPv4
    V4,
    /// Use only IPv6
    V6,
    /// Use both IPv4 and IPv6 (default)
    #[serde(rename = "both")]
    Both,
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
