// Standard library
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

// 3rd party crates
use async_trait::async_trait;

/// Core trait that all DNS providers must implement.
/// This trait defines the basic operations required for a DNS provider
/// to update DNS records with IPv4 and IPv6 addresses.
///
/// # Implementation Requirements
///
/// Implementors must provide:
/// - Configuration type that defines provider-specific settings
/// - Error type for provider-specific errors
/// - Methods to update DNS records for both IPv4 and IPv6
/// - Configuration validation
/// - Enable/disable functionality
///
/// # Example Implementation
///
/// ```rust
/// # use async_trait::async_trait;
/// # use std::net::{Ipv4Addr, Ipv6Addr};
/// #[derive(Clone)]
/// struct MyProviderConfig {
///     api_key: String,
///     domains: Vec<String>,
/// }
///
/// struct MyProvider {
///     config: MyProviderConfig,
/// }
///
/// #[async_trait]
/// impl DnsProvider for MyProvider {
///     type Config = MyProviderConfig;
///     type Error = Box<dyn std::error::Error + Send + Sync>;
///
///     fn new(config: Self::Config) -> Result<Self, Self::Error> {
///         Ok(Self { config })
///     }
///
///     async fn update_dns_records_v4(&self, ip: &Ipv4Addr) -> Result<(), Self::Error> {
///         // Update A records
///         Ok(())
///     }
///
///     async fn update_dns_records_v6(&self, ip: &Ipv6Addr) -> Result<(), Self::Error> {
///         // Update AAAA records
///         Ok(())
///     }
///
///     fn validate_config(&self) -> Result<(), Self::Error> {
///         // Validate API key and domains
///         Ok(())
///     }
///
///     fn is_enabled(&self) -> bool {
///         true
///     }
///
///     fn get_name(&self) -> &str {
///         "my_provider"
///     }
/// }
/// ```
#[async_trait]
#[allow(unused)]
pub trait DnsProvider: Send + Sync {
    /// The configuration type for this provider.
    ///
    /// This type should contain all necessary settings for the provider,
    /// such as API credentials, domain lists, and provider-specific options.
    /// It must be Clone to allow sharing configuration across async tasks.
    type Config: Clone + Send + Sync;

    /// The error type for this provider.
    ///
    /// This type should encompass all possible errors that can occur during
    /// provider operations, including API errors, validation errors, and
    /// network errors.
    type Error: std::error::Error + Send + Sync;

    /// Creates a new instance of the DNS provider with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Provider-specific configuration
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Successfully initialized provider
    /// * `Err(Self::Error)` - Configuration or initialization error
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::error::Error;
    /// # struct MyProvider;
    /// # struct Config;
    /// # impl MyProvider {
    /// #     fn new(config: Config) -> Result<Self, Box<dyn Error + Send + Sync>> {
    /// let config = Config { /* ... */ };
    /// let provider = MyProvider::new(config)?;
    /// #         Ok(MyProvider)
    /// #     }
    /// # }
    /// ```
    fn new(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Updates DNS A records for all configured domains with the given IPv4 address.
    ///
    /// This method should handle:
    /// - API rate limiting
    /// - Retries on temporary failures
    /// - Validation of DNS record updates
    ///
    /// # Arguments
    ///
    /// * `ip` - The IPv4 address to set in DNS records
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All records updated successfully
    /// * `Err(Self::Error)` - Update failed (partially or completely)
    async fn update_dns_records_v4(&self, ip: &Ipv4Addr) -> Result<(), Self::Error>;

    /// Updates DNS AAAA records for all configured domains with the given IPv6 address.
    ///
    /// This method should handle:
    /// - API rate limiting
    /// - Retries on temporary failures
    /// - Validation of DNS record updates
    ///
    /// # Arguments
    ///
    /// * `ip` - The IPv6 address to set in DNS records
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All records updated successfully
    /// * `Err(Self::Error)` - Update failed (partially or completely)
    async fn update_dns_records_v6(&self, ip: &Ipv6Addr) -> Result<(), Self::Error>;

    /// Updates DNS records for all configured domains with either IPv4 or IPv6 address.
    ///
    /// This is a convenience method that delegates to either `update_dns_records_v4`
    /// or `update_dns_records_v6` based on the IP address type.
    ///
    /// # Arguments
    ///
    /// * `ip` - The IP address (either v4 or v6) to set in DNS records
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All records updated successfully
    /// * `Err(Self::Error)` - Update failed (partially or completely)
    async fn update_dns_records_ip(&self, ip: &IpAddr) -> Result<(), Self::Error> {
        match ip {
            IpAddr::V4(ipv4) => self.update_dns_records_v4(ipv4).await,
            IpAddr::V6(ipv6) => self.update_dns_records_v6(ipv6).await,
        }
    }

    /// Validates the provider's configuration.
    ///
    /// This method should check:
    /// - Required fields are present
    /// - Credentials are well-formed
    /// - Domain names are valid
    /// - Provider-specific requirements are met
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Configuration is valid
    /// * `Err(Self::Error)` - Configuration is invalid
    fn validate_config(&self) -> Result<(), Self::Error>;

    /// Checks if the provider is enabled.
    ///
    /// This allows providers to be conditionally enabled/disabled
    /// without removing their configuration.
    ///
    /// # Returns
    ///
    /// * `true` - Provider is enabled and should be used
    /// * `false` - Provider is disabled and should be skipped
    fn is_enabled(&self) -> bool;

    /// Gets the provider's name.
    ///
    /// This name should be:
    /// - Lowercase
    /// - No spaces
    /// - Unique across all providers
    ///
    /// # Returns
    ///
    /// A string slice containing the provider name
    fn get_name(&self) -> &str;
}
