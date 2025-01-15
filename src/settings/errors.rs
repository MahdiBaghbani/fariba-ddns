// 3rd party crates
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid log level: {0}. Must be one of: error, warn, info, debug, trace")]
    InvalidLogLevel(String),
    #[error("Update interval must be greater than 0, got {0}")]
    InvalidUpdateInterval(u64),
    #[error("No providers are enabled")]
    NoProvidersEnabled,
    #[error("Invalid Cloudflare configuration: {0}")]
    CloudflareConfig(String),
    #[error("Invalid IP detection configuration: {0}")]
    IpDetectionConfig(String),
}
