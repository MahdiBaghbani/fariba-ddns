# User Guide

This guide will help you get started with Fariba DDNS Client and explain how to use its features effectively.

## Installation

### From Source
```bash
git clone https://github.com/yourusername/fariba-ddns
cd fariba-ddns
cargo install --path .
```

### Using Docker
```bash
docker-compose up -d
```

## Basic Usage

1. **Initial Setup**
   ```bash
   cp example.toml .settings.toml
   ```

2. **Configuration**
   Edit `.settings.toml` with your preferred text editor and configure:
   - DNS provider credentials
   - Domains to update
   - Update intervals
   - IP detection settings

3. **Running the Client**
   ```bash
   # Run with default config file (.settings.toml)
   fariba-ddns

   # Run with custom config file
   fariba-ddns --config path/to/config.toml
   ```

## Features

### IP Detection
The client supports multiple IP detection methods and uses consensus to ensure accuracy:
- Multiple public IP detection services
- Local network interface detection
- IPv4 and IPv6 support

### DNS Providers
Currently supported providers:
- Cloudflare
- ArvanCloud
(See [Provider Setup](providers.md) for configuration details)

### Automatic Updates
- Configurable update intervals
- Smart update detection (only updates when IP changes)
- Rate limiting to respect provider API limits

### Logging
- Detailed logging with configurable levels
- Error reporting and troubleshooting information

## Troubleshooting

### Common Issues

1. **Configuration Errors**
   - Check your provider credentials
   - Verify domain permissions
   - Ensure correct domain/subdomain format

2. **Network Issues**
   - Check internet connectivity
   - Verify firewall settings
   - Ensure DNS provider APIs are accessible

3. **IP Detection Problems**
   - Try different IP detection services
   - Check network interface configuration
   - Verify IPv6 support if needed

### Logs
Logs are written to stdout by default. Use your system's logging infrastructure to capture and analyze them.

## Advanced Usage

### Environment Variables
You can override configuration using environment variables:
```bash
DDNS_LOG_LEVEL=debug fariba-ddns
```

### Custom Configuration
See [Configuration Guide](configuration.md) for detailed configuration options. 