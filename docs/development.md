# Development Guide

This guide is for developers who want to contribute to Fariba DDNS Client.

## Development Environment

### Prerequisites
- Rust 1.70 or higher
- Cargo
- Git
- (Optional) Docker for containerized testing

### Setup

1. **Clone the Repository**
   ```bash
   git clone https://github.com/yourusername/fariba-ddns
   cd fariba-ddns
   ```

2. **Install Development Dependencies**
   ```bash
   # Install cargo tools
   cargo install cargo-watch cargo-audit
   ```

3. **Configure Development Environment**
   ```bash
   cp example.toml .settings.toml
   cp env.example .env
   ```

## Project Structure

```
fariba-ddns/
├── src/
│   ├── main.rs           # Application entry point
│   ├── functions.rs      # Core functionality
│   ├── providers/        # DNS provider implementations
│   │   ├── cloudflare/
│   │   └── arvancloud/
│   ├── settings/        # Configuration management
│   └── utility/         # Shared utilities
├── tests/              # Integration tests
├── docs/              # Documentation
└── docker/            # Docker-related files
```

## Development Workflow

### Building
```bash
# Development build
cargo build

# Release build
cargo build --release

# Watch mode (auto-rebuild on changes)
cargo watch -x build
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with logging
RUST_LOG=debug cargo test
```

### Code Style
- Follow Rust standard formatting (`cargo fmt`)
- Use `cargo clippy` for linting
- Document public APIs with rustdoc comments
- Write unit tests for new functionality

## Adding a New Provider

1. Create a new module in `src/providers/`:
   ```rust
   providers/
   └── new_provider/
       ├── mod.rs
       ├── types.rs
       ├── errors.rs
       ├── functions.rs
       └── impls.rs
   ```

2. Implement the Provider trait:
   ```rust
   use async_trait::async_trait;
   use crate::providers::traits::Provider;

   #[async_trait]
   impl Provider for NewProvider {
       async fn update_records(&self, ip: &str) -> Result<(), Error>;
       // ... other trait methods
   }
   ```

3. Add configuration support in `settings/`:
   ```rust
   #[derive(Debug, Deserialize)]
   pub struct NewProviderConfig {
       pub api_key: String,
       pub domains: Vec<String>,
   }
   ```

4. Add tests in `tests/providers/`

## Testing

### Unit Tests
- Write tests for all public functions
- Use mock objects for external services
- Test error conditions

### Integration Tests
- Add tests in `tests/` directory
- Test complete workflows
- Use test fixtures for configuration

### Example Test
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_update() {
        let config = NewProviderConfig {
            api_key: "test_key".to_string(),
            domains: vec!["example.com".to_string()],
        };
        let provider = NewProvider::new(config);
        let result = provider.update_records("1.2.3.4").await;
        assert!(result.is_ok());
    }
}
```

## Documentation

### Code Documentation
- Use rustdoc comments (`///`) for public items
- Include examples in documentation
- Document error conditions
- Cross-reference related items

### Example Documentation
```rust
/// Updates DNS records for the configured domains.
///
/// # Arguments
///
/// * `ip` - The IP address to set in the DNS records
///
/// # Returns
///
/// * `Ok(())` if the update was successful
/// * `Err(Error)` if the update failed
///
/// # Examples
///
/// ```
/// # async fn example() -> Result<(), Error> {
/// let provider = NewProvider::new(config);
/// provider.update_records("1.2.3.4").await?;
/// # Ok(())
/// # }
/// ```
pub async fn update_records(&self, ip: &str) -> Result<(), Error>
```

## Release Process

1. Update version in `Cargo.toml`
2. Update CHANGELOG.md
3. Run full test suite
4. Create git tag
5. Build release artifacts
6. Update documentation

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Update documentation
6. Submit a pull request

See [CONTRIBUTING.md](../CONTRIBUTING.md) for detailed guidelines. 