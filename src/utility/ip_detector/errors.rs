// 3rd party crates
use thiserror::Error;

// Current module imports
use super::types::IpVersion;

#[derive(Debug, Error)]
pub enum IpDetectionError {
    #[error("Network error from {service}: {error}")]
    NetworkError {
        service: String,
        error: reqwest::Error,
    },

    #[error("Invalid response from {service}: {response}")]
    InvalidResponse { service: String, response: String },

    #[error("IP version mismatch from {service}: expected {expected:?}, got {got:?}")]
    VersionMismatch {
        service: String,
        expected: IpVersion,
        got: IpVersion,
    },

    #[error("Rate limit exceeded for {service}")]
    RateLimitExceeded { service: String },

    #[error("Parse error from {service}: {error}")]
    ParseError { service: String, error: String },

    #[error("Consensus not reached: got {responses} responses, need {required}")]
    ConsensusNotReached { responses: usize, required: u32 },

    #[error("No IP detection services available")]
    NoServicesAvailable,

    #[error("{version:?} detection suspended for {remaining_secs} seconds")]
    VersionSuspended {
        version: IpVersion,
        remaining_secs: u64,
    },

    #[error("Validation error: {0}")]
    Validation(#[from] IpDetectionValidationError),
}

#[derive(Debug, Error)]
pub enum IpDetectionValidationError {
    #[error("Invalid max_requests_per_hour: {0}")]
    InvalidMaxRequests(String),
    #[error("Invalid min_consensus: {0}")]
    InvalidMinConsensus(String),
    #[error("Invalid network_retry_interval: {0}")]
    InvalidRetryInterval(String),
}
