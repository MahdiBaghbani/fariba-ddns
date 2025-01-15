# Fariba DDNS Client

A flexible Dynamic DNS client written in Rust that supports multiple DNS providers for homelab and self-hosted services.

## Features

- Multiple DNS provider support
- Automatic IP detection
- Configurable update intervals
- Docker support
- Detailed logging

## Quick Start

### Installation

```bash
# From source
git clone https://github.com/yourusername/fariba-ddns
cd fariba-ddns
cargo install --path .

# Using Docker
docker-compose up -d
```

### Configuration

1. Copy the example configuration:
```bash
cp example.toml .settings.toml
```

2. Edit `.settings.toml` with your provider credentials and settings.

See [Configuration Guide](docs/configuration.md) for detailed settings.

## Usage

```bash
# Run directly
fariba-ddns

# With custom config path
fariba-ddns --config path/to/config.toml
```

## Documentation

- [User Guide](docs/user-guide.md) - Complete guide for end users
- [Configuration](docs/configuration.md) - Detailed configuration options
- [Provider Setup](docs/providers.md) - Provider-specific setup instructions
- [Development Guide](docs/development.md) - Guide for contributors
- [Architecture](docs/architecture.md) - System design and components
- [API Documentation](https://docs.rs/fariba-ddns) - Rust API documentation

## Development

### Prerequisites

- Rust 1.70 or higher
- Cargo

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test
```

## Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under the GNU Affero General Public License v3.0 - see the [LICENSE](LICENSE) file for details. 