// Standard library
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

// 3rd party crates
use async_trait::async_trait;

/// Core trait that all DNS providers must implement.
/// This trait defines the basic operations required for a DNS provider
/// to update DNS records with IPv4 and IPv6 addresses.
#[async_trait]
#[allow(unused)]
pub trait DnsProvider: Send + Sync {
    /// The specific configuration type for this provider
    type Config: Clone + Send + Sync;
    /// The specific error type for this provider
    type Error: std::error::Error + Send + Sync;

    /// Creates a new instance of the DNS provider with the given configuration
    fn new(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Updates DNS records for all configured subdomains with IPv4
    async fn update_dns_records_v4(&self, ip: &Ipv4Addr) -> Result<(), Self::Error>;

    /// Updates DNS records for all configured subdomains with IPv6
    async fn update_dns_records_v6(&self, ip: &Ipv6Addr) -> Result<(), Self::Error>;

    /// Updates DNS records for all configured subdomains with either IPv4 or IPv6.
    /// This is a convenience method that delegates to the appropriate specific method.
    async fn update_dns_records_ip(&self, ip: &IpAddr) -> Result<(), Self::Error> {
        match ip {
            IpAddr::V4(ipv4) => self.update_dns_records_v4(ipv4).await,
            IpAddr::V6(ipv6) => self.update_dns_records_v6(ipv6).await,
        }
    }

    /// Validates the provider's configuration
    fn validate_config(&self) -> Result<(), Self::Error>;

    /// Checks if the provider is enabled
    fn is_enabled(&self) -> bool;

    /// Gets the provider name
    fn get_name(&self) -> &str;
}
