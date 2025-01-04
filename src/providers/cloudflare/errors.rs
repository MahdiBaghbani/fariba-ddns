use reqwest::{header::InvalidHeaderValue, StatusCode};
use thiserror::Error;

/// Custom error type for Cloudflare operations.
#[derive(Error, Debug)]
pub enum CloudflareError {
    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    #[error("Failed to build HTTP client: {0}")]
    HttpClientBuild(#[source] reqwest::Error),

    #[error("Invalid API token for '{0}'")]
    InvalidApiToken(String),

    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    #[error("Zone '{0}' is not active (status: {1})")]
    InactiveZone(String, String),

    #[error("Failed to parse response for '{0}': {1}")]
    ParsingError(String, #[source] reqwest::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("HTTP error for '{zone}': status code {status}, message: {message}")]
    HttpStatusError {
        zone: String,
        status: StatusCode,
        message: String,
    },

    #[error("API response indicates failure for zone '{0}'")]
    ApiResponseFailure(String),
}
