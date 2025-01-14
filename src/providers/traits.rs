// Standard library
use std::net::Ipv4Addr;

// 3rd party crates
use async_trait::async_trait;
use serde::Deserialize;

/// Represents a generic DNS record update configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DnsUpdateConfig {
    pub enabled: bool,
    pub name: String,
    pub subdomains: Vec<String>,
}

/// Core trait that all DNS providers must implement
#[async_trait]
pub trait DnsProvider: Send + Sync {
    /// The specific configuration type for this provider
    type Config: Clone + Send + Sync;
    /// The specific error type for this provider
    type Error: std::error::Error + Send + Sync;

    /// Create a new instance of the DNS provider
    fn new(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Update DNS records for all configured subdomains
    async fn update_dns_records(&self, ip: &Ipv4Addr) -> Result<(), Self::Error>;

    /// Validate the provider's configuration
    fn validate_config(&self) -> Result<(), Self::Error>;

    /// Check if the provider is enabled
    fn is_enabled(&self) -> bool;

    /// Get the provider name
    fn get_name(&self) -> &str;
}
