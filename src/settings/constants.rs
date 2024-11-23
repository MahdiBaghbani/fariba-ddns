/// The default configuration file content with comments to guide the user.
pub const DEFAULT_CONFIG: &str = r#"
# Logging configuration
[log]
# Level can be "error", "warn", "info", "debug", or "trace"
level = "trace"

# Update interval in seconds
[update]
interval = 300

# Cloudflare configurations
[[cloudflare]]
enabled = true
name = "example.com"
zone_id = "your_zone_id_here"
api_token = "your_api_token_here"

# List of subdomains to update
[[cloudflare.subdomains]]
name = "www"

[[cloudflare.subdomains]]
name = "api"
"#;
