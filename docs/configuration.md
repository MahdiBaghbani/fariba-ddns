# Configuration Guide

This guide explains all configuration options available in Fariba DDNS Client.

## Configuration File

The configuration file uses TOML format. By default, the client looks for `.settings.toml` in the current directory.

## Basic Structure

```toml
[general]
update_interval = 300  # seconds
log_level = "info"    # debug, info, warn, error

[ip_detection]
services = ["ipify", "cloudflare"]
consensus_threshold = 2

[providers.cloudflare]
api_token = "your-api-token"
zone_id = "your-zone-id"
domains = ["example.com", "*.example.com"]

[providers.arvancloud]
api_key = "your-api-key"
domains = ["example.ir"]
```

## Configuration Sections

### General Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| update_interval | integer | 300 | Time between IP checks (seconds) |
| log_level | string | "info" | Logging verbosity |

### IP Detection

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| services | array | ["ipify"] | IP detection services to use |
| consensus_threshold | integer | 2 | Minimum services that must agree |
| timeout | integer | 10 | Service timeout in seconds |

### Provider Settings

#### Cloudflare

| Option | Type | Required | Description |
|--------|------|----------|-------------|
| api_token | string | Yes | Cloudflare API token |
| zone_id | string | Yes | DNS zone ID |
| domains | array | Yes | Domains to update |

#### ArvanCloud

| Option | Type | Required | Description |
|--------|------|----------|-------------|
| api_key | string | Yes | ArvanCloud API key |
| domains | array | Yes | Domains to update |

## Environment Variables

All configuration options can be overridden using environment variables:

| Environment Variable | Configuration Equivalent |
|---------------------|-------------------------|
| DDNS_UPDATE_INTERVAL | general.update_interval |
| DDNS_LOG_LEVEL | general.log_level |
| DDNS_CF_API_TOKEN | providers.cloudflare.api_token |
| DDNS_CF_ZONE_ID | providers.cloudflare.zone_id |
| DDNS_ARVAN_API_KEY | providers.arvancloud.api_key |

## Example Configurations

### Minimal Configuration
```toml
[providers.cloudflare]
api_token = "your-token"
zone_id = "your-zone"
domains = ["example.com"]
```

### Full Configuration
```toml
[general]
update_interval = 300
log_level = "debug"

[ip_detection]
services = ["ipify", "cloudflare", "local"]
consensus_threshold = 2
timeout = 15

[providers.cloudflare]
api_token = "your-token"
zone_id = "your-zone"
domains = ["example.com", "*.example.com"]

[providers.arvancloud]
api_key = "your-key"
domains = ["example.ir"]
```

## Security Considerations

- Store sensitive credentials in environment variables for production use
- Use restricted API tokens with minimum required permissions
- Keep your configuration file secure and private 