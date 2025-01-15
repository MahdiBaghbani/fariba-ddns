/// Example configuration
pub const DEFAULT_CONFIG: &str = r#"
# Logging configuration
[log]
# Level can be "error", "warn", "info", "debug", or "trace"
level = "trace"

# Update interval in seconds
[update]
interval = 300

# Cloudflare provider configuration
[[cloudflare]]
enabled = true
name = "example"
zone_id = "your_zone_id"
api_token = "your_api_token"

# Rate limiting configuration (optional)
rate_limit = { max_requests = 30, window_secs = 60 }

# List of subdomains to update
[[cloudflare.subdomains]]
name = "www"
# Optional: specify which IP versions to use (v4, v6, or both)
# Default is "both" if not specified
ip_version = "both"

[[cloudflare.subdomains]]
name = "ipv4-only"
ip_version = "v4"

[[cloudflare.subdomains]]
name = "ipv6-only"
ip_version = "v6"

[[cloudflare.subdomains]]
# Empty name means root domain
name = ""
ip_version = "both"
"#;
