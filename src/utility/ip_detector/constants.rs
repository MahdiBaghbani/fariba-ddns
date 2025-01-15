// Project imports
use crate::utility::ip_detector::types::IpService;

/// Number of primary IP detection services
pub const PRIMARY_SERVICE_COUNT: usize = 3;

/// Default settings
pub const DEFAULT_MAX_REQUESTS_PER_HOUR: u32 = 200;
pub const DEFAULT_MIN_CONSENSUS: u32 = 3;
pub const DEFAULT_MAX_NETWORK_RETRY_INTERVAL: u64 = 30;

/// HTTP client settings
pub const REQUEST_TIMEOUT_SECS: u64 = 5;
pub const MAX_RETRIES: u32 = 2;
pub const RETRY_DELAY_MS: u64 = 500;

/// IPv4 detection services
pub const IPV4_SERVICES: [IpService; 4] = [
    // Primary services
    IpService {
        base_url: "https://api.ipify.org",
        path: "?format=text",
        is_primary: true,
    },
    IpService {
        base_url: "https://v4.ident.me",
        path: "",
        is_primary: true,
    },
    IpService {
        base_url: "https://api4.my-ip.io",
        path: "/v2/ip.txt",
        is_primary: true,
    },
    // Secondary services
    IpService {
        base_url: "https://ipv4.icanhazip.com",
        path: "",
        is_primary: false,
    },
];

/// IPv6 detection services
pub const IPV6_SERVICES: [IpService; 4] = [
    // Primary services
    IpService {
        base_url: "https://api6.ipify.org",
        path: "?format=text",
        is_primary: true,
    },
    IpService {
        base_url: "https://v6.ident.me",
        path: "",
        is_primary: true,
    },
    IpService {
        base_url: "https://api6.my-ip.io",
        path: "/ip",
        is_primary: true,
    },
    // Secondary services
    IpService {
        base_url: "https://ipv6.icanhazip.com",
        path: "",
        is_primary: false,
    },
];

pub fn default_max_requests_per_hour() -> u32 {
    DEFAULT_MAX_REQUESTS_PER_HOUR
}

pub fn default_min_consensus() -> u32 {
    DEFAULT_MIN_CONSENSUS
}

pub fn default_network_retry_interval() -> u64 {
    DEFAULT_MAX_NETWORK_RETRY_INTERVAL
}
