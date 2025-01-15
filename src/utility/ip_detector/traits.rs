// Project imports
use super::types::{IpService, IpVersion};

/// Trait for IP version-specific operations
pub trait IpVersionOps {
    /// Get the services for this IP version
    fn get_services() -> &'static [IpService];
    /// Get the rate limiter offset for this IP version
    fn rate_limiter_offset() -> usize;
    /// Get the version enum for this IP version
    fn version() -> IpVersion;
}
