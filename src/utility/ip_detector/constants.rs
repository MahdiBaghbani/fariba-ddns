// Project imports
use crate::utility::ip_detector::types::IpService;

/// List of IP detection services with their configurations

pub const IP_SERVICES: &[IpService] = &[
    IpService {
        base_url: "https://api.ipify.org",
        v4_path: "?format=text",
        v6_path: "?format=text",
        supports_v6: true,
    },
    IpService {
        base_url: "https://api4.ipify.org",
        v4_path: "?format=text",
        v6_path: "",
        supports_v6: false,
    },
    IpService {
        base_url: "https://api6.ipify.org",
        v4_path: "",
        v6_path: "?format=text",
        supports_v6: true,
    },
    IpService {
        base_url: "https://ifconfig.me",
        v4_path: "/ip",
        v6_path: "/ip",
        supports_v6: true,
    },
    IpService {
        base_url: "https://icanhazip.com",
        v4_path: "",
        v6_path: "",
        supports_v6: true,
    },
    IpService {
        base_url: "https://ip.seeip.org",
        v4_path: "/json",
        v6_path: "/jsonip",
        supports_v6: true,
    },
    IpService {
        base_url: "https://api.my-ip.io",
        v4_path: "/ip",
        v6_path: "/ip",
        supports_v6: true,
    },
];
