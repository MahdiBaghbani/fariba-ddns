// Standard library
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

// 3rd party crates
use serde::Deserialize;
use tokio::sync::RwLock;

// Project imports
use crate::utility::rate_limiter::traits::RateLimiter;

use super::constants::{
    default_max_requests_per_hour, default_min_consensus, default_network_retry_interval,
};

#[derive(Debug, Deserialize, Clone)]
pub struct IpDetection {
    /// Maximum requests per hour to each IP detection service
    #[serde(default = "default_max_requests_per_hour")]
    pub max_requests_per_hour: u32,
    /// Minimum number of services that must agree on the IP
    #[serde(default = "default_min_consensus")]
    pub min_consensus: u32,
    /// Network check interval when connectivity is lost (in seconds)
    #[serde(default = "default_network_retry_interval")]
    pub network_retry_interval: u64,
}

/// Suspension state for an IP version
#[derive(Debug)]
pub struct VersionSuspension {
    pub suspended_since: Instant,
    pub consecutive_failures: u32,
}

pub struct IpDetector {
    pub config: IpDetection,
    pub rate_limiters: Vec<Arc<dyn RateLimiter>>,
    pub client: reqwest::Client,
    pub suspended_versions: Arc<RwLock<HashMap<IpVersion, VersionSuspension>>>,
}

/// Service configuration for IP detection
pub struct IpService {
    pub base_url: &'static str,
    pub path: &'static str,
    pub is_primary: bool,
}

#[derive(Debug)]
#[allow(unused)]
pub struct IpResponse {
    pub ip: IpAddr,
    pub is_primary: bool,
}

/// IPv4 version operations
pub struct V4;

/// IPv6 version operations
pub struct V6;

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum IpVersion {
    V4,
    V6,
}
