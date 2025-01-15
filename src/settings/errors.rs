// 3rd party crates
use thiserror::Error;

// Project imports
use crate::providers::cloudflare::errors::CloudflareValidationError;
use crate::utility::ip_detector::errors::IpDetectionValidationError;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Invalid log level: {0}. Must be one of: error, warn, info, debug, trace")]
    InvalidLogLevel(String),
    #[error("Update interval must be greater than 0, got {0}")]
    InvalidUpdateInterval(u64),
    #[error("No providers are enabled")]
    NoProvidersEnabled,
    #[error("Cloudflare configuration error: {0}")]
    CloudflareConfig(#[from] CloudflareValidationError),
    #[error("IP detection configuration error: {0}")]
    IpDetectionConfig(#[from] IpDetectionValidationError),
}
