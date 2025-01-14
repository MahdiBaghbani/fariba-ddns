// 3rd party crates
use thiserror::Error;

/// Custom error type for Cloudflare operations.
#[derive(Debug, Error)]
pub enum CloudflareError {
    #[error("Invalid API token for zone '{0}'")]
    InvalidApiToken(String),

    #[error("Invalid zone ID for zone '{0}'")]
    InvalidZoneId(String),

    #[error("No subdomains configured for zone '{0}'")]
    NoSubdomains(String),

    #[error("HTTP client error: {0}")]
    HttpClientBuild(#[from] reqwest::Error),

    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error("Failed to update DNS records for zone '{zone}': {message}")]
    UpdateFailed { zone: String, message: String },

    #[error("Failed to fetch DNS records for zone '{zone}': {message}")]
    FetchFailed { zone: String, message: String },

    #[error("Rate limit exceeded for zone '{0}'")]
    RateLimited(String),

    #[error("Invalid rate limit configuration for zone '{zone}': {reason}")]
    InvalidRateLimit { zone: String, reason: String },

    #[error("Invalid subdomain '{subdomain}' for zone '{zone}'")]
    InvalidSubdomain { zone: String, subdomain: String },

    #[error("Zone '{0}' is not active (status: {1})")]
    InactiveZone(String, String),
}
