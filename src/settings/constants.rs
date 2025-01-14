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

# Subdomains to update
subdomains = [
    { name = "example.com" },
    { name = "subdomain.example.com" }
]
"#;
