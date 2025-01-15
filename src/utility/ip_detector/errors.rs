// 3rd party crates
use reqwest;

// Project imports
use super::types::IpVersion;

/// Custom error type for IP detection
#[derive(Debug)]
pub enum IpDetectionError {
    /// Network error when contacting service
    NetworkError {
        service: String,
        error: reqwest::Error,
    },
    /// Invalid response from service
    InvalidResponse { service: String, response: String },
    /// IP version mismatch
    VersionMismatch {
        service: String,
        expected: IpVersion,
        got: IpVersion,
    },
    /// Rate limit exceeded
    RateLimitExceeded { service: String },
    /// Parsing error
    ParseError { service: String, error: String },
    /// Consensus not reached
    ConsensusNotReached { responses: usize, required: u32 },
    /// No services available
    NoServicesAvailable,
    /// Version suspended
    VersionSuspended {
        version: IpVersion,
        remaining_secs: u64,
    },
}
