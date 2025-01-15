// Project imports
use crate::utility::ip_detector::types::IpService;

/// Number of primary IP detection services
pub const PRIMARY_SERVICE_COUNT: usize = 3;

/// Default settings
pub const DEFAULT_MAX_REQUESTS_PER_HOUR: u32 = 200;
pub const DEFAULT_MIN_CONSENSUS: u32 = 4;
pub const DEFAULT_MAX_NETWORK_RETRY_INTERVAL: u64 = 30;

/// HTTP client settings
pub const REQUEST_TIMEOUT_SECS: u64 = 5;
pub const MAX_RETRIES: u32 = 2;
pub const RETRY_DELAY_MS: u64 = 500;

/// IPv4 detection services
pub const IPV4_SERVICES: [IpService; 12] = [
    // Primary services (highly reliable)
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
    // Secondary services (reliable backups)
    IpService {
        base_url: "https://ipv4.icanhazip.com",
        path: "",
        is_primary: false,
    },
    IpService {
        base_url: "https://ip4.seeip.org",
        path: "",
        is_primary: false,
    },
    IpService {
        base_url: "https://api4.ipaddress.com",
        path: "/myip",
        is_primary: false,
    },
    IpService {
        base_url: "https://ipecho.net",
        path: "/plain",
        is_primary: false,
    },
    IpService {
        base_url: "https://checkip.amazonaws.com",
        path: "",
        is_primary: false,
    },
    IpService {
        base_url: "https://ipinfo.io",
        path: "/ip",
        is_primary: false,
    },
    IpService {
        base_url: "https://wtfismyip.com",
        path: "/text",
        is_primary: false,
    },
    IpService {
        base_url: "https://ip.tyk.nu",
        path: "",
        is_primary: false,
    },
    IpService {
        base_url: "https://diagnostic.opendns.com",
        path: "/myip",
        is_primary: false,
    },
];

/// IPv6 detection services
pub const IPV6_SERVICES: [IpService; 10] = [
    // Primary services (highly reliable)
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
    // Secondary services (reliable backups)
    IpService {
        base_url: "https://ipv6.icanhazip.com",
        path: "",
        is_primary: false,
    },
    IpService {
        base_url: "https://ip6.seeip.org",
        path: "",
        is_primary: false,
    },
    IpService {
        base_url: "https://v6.ipv6-test.com",
        path: "/api/myip.php",
        is_primary: false,
    },
    IpService {
        base_url: "https://ipv6.wtfismyip.com",
        path: "/text",
        is_primary: false,
    },
    IpService {
        base_url: "https://ipv6.ip.tyk.nu",
        path: "",
        is_primary: false,
    },
    IpService {
        base_url: "https://v6.ident.me",
        path: "/raw",
        is_primary: false,
    },
    IpService {
        base_url: "https://ipv6.test-ipv6.com",
        path: "/ip/",
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
