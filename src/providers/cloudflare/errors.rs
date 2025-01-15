// 3rd party crates
use thiserror::Error;

/// Represents errors that can occur during Cloudflare API operations
/// These variants are retained for comprehensive error handling of all possible
/// API failure modes, even if some paths are not currently exercised.
/// This ensures forward compatibility and proper error handling for future
/// implementations and edge cases.
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

    #[error("Failed to create DNS record for domain '{domain}' in zone '{zone}': {message}")]
    CreateFailed {
        zone: String,
        domain: String,
        message: String,
    },

    #[error("Rate limit exceeded for zone '{0}'")]
    RateLimited(String),

    #[error("Invalid rate limit configuration for zone '{zone}': {reason}")]
    InvalidRateLimit { zone: String, reason: String },

    #[error("Zone '{0}' is not active (status: {1})")]
    InactiveZone(String, String),

    #[error("Operation timed out for zone '{zone}': {message}")]
    Timeout { zone: String, message: String },

    #[error("DNS update operation timed out")]
    UpdateTimeout,

    #[error("Validation error: {0}")]
    Validation(#[from] CloudflareValidationError),
}

#[derive(Debug, Error)]
pub enum CloudflareValidationError {
    #[error("Missing or empty zone_id")]
    MissingZoneId,
    #[error("Missing or empty api_token")]
    MissingApiToken,
    #[error("Missing or empty name")]
    MissingName,
    #[error("No subdomains configured")]
    NoSubdomains,
    #[error("Invalid rate limit: {0}")]
    InvalidRateLimit(String),
    #[error("Invalid IP version configuration: {0}")]
    InvalidIpVersion(String),
}
