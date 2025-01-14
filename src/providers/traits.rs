// Standard library
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

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

    /// Update DNS records for all configured subdomains with IPv4
    async fn update_dns_records(&self, ip: &Ipv4Addr) -> Result<(), Self::Error>;

    /// Update DNS records for all configured subdomains with IPv6
    async fn update_dns_records_v6(&self, ip: &Ipv6Addr) -> Result<(), Self::Error>;

    /// Update DNS records for all configured subdomains with either IPv4 or IPv6
    async fn update_dns_records_ip(&self, ip: &IpAddr) -> Result<(), Self::Error> {
        match ip {
            IpAddr::V4(ipv4) => self.update_dns_records(ipv4).await,
            IpAddr::V6(ipv6) => self.update_dns_records_v6(ipv6).await,
        }
    }

    /// Validate the provider's configuration
    fn validate_config(&self) -> Result<(), Self::Error>;

    /// Check if the provider is enabled
    fn is_enabled(&self) -> bool;

    /// Get the provider name
    fn get_name(&self) -> &str;
}
