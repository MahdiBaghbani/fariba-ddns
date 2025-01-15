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

/// IP services for detecting public IP addresses
pub const IP_SERVICES: [IpService; 8] = [
    // Primary services (more reliable)
    IpService {
        base_url: "https://api64.ipify.org",
        v4_path: "?format=text",
        v6_path: "?format=text",
        supports_v6: true,
        is_primary: true,
    },
    IpService {
        base_url: "https://v4.ident.me",
        v4_path: "",
        v6_path: "",
        supports_v6: false,
        is_primary: true,
    },
    IpService {
        base_url: "https://v6.ident.me",
        v4_path: "",
        v6_path: "",
        supports_v6: true,
        is_primary: true,
    },
    // Secondary services (backup)
    IpService {
        base_url: "https://api6.my-ip.io",
        v4_path: "/ip",
        v6_path: "/ip",
        supports_v6: true,
        is_primary: false,
    },
    IpService {
        base_url: "https://ipv6.icanhazip.com",
        v4_path: "",
        v6_path: "",
        supports_v6: true,
        is_primary: false,
    },
    IpService {
        base_url: "https://ipv4.icanhazip.com",
        v4_path: "",
        v6_path: "",
        supports_v6: false,
        is_primary: false,
    },
    IpService {
        base_url: "https://v4.ipv6-test.com",
        v4_path: "/api/myip.php",
        v6_path: "",
        supports_v6: false,
        is_primary: false,
    },
    IpService {
        base_url: "https://v6.ipv6-test.com",
        v4_path: "",
        v6_path: "/api/myip.php",
        supports_v6: true,
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
