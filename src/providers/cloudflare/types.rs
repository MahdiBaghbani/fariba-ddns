// Standard library
use std::net::Ipv4Addr;

// 3rd party crates
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

// Project modules
use crate::providers::DnsProvider;

use super::errors::CloudflareError;

/// Represents a client for interacting with the Cloudflare API.
#[derive(Debug, Clone)]
pub struct Cloudflare {
    pub config: CfConfig,
    pub client: Client,
}

/// Configuration for Cloudflare API interactions.
#[derive(Debug, Deserialize, Clone)]
pub struct CfConfig {
    pub enabled: bool,
    pub name: String,
    pub zone_id: String,
    pub api_token: String,
    pub subdomains: Vec<CfSubDomain>,
}

/// Represents a subdomain configuration in Cloudflare.
#[derive(Debug, Deserialize, Clone)]
pub struct CfSubDomain {
    pub name: String,
}

/// Represents the response from a DNS record request.
#[derive(Debug, Deserialize)]
pub struct DnsResponse {
    pub result: Vec<DnsResponseResult>,
}

/// Details of the DNS response result.
#[derive(Debug, Deserialize)]
pub struct DnsResponseResult {
    pub id: String,
    pub content: String,
}

/// Represents the response from a zone request.
#[derive(Debug, Deserialize)]
pub struct ZoneResponse {
    pub result: ZoneResponseResult,
    pub success: bool,
}

/// Details of the zone response result.
#[derive(Debug, Deserialize)]
pub struct ZoneResponseResult {
    pub status: String,
}

#[async_trait]
impl DnsProvider for Cloudflare {
    type Config = CfConfig;
    type Error = CloudflareError;

    fn new(config: Self::Config) -> Result<Self, Self::Error> {
        use super::functions::create_reqwest_client;
        let client = create_reqwest_client(&config)?;
        Ok(Self { config, client })
    }

    async fn update_dns_records(&self, ip: &Ipv4Addr) -> Result<(), Self::Error> {
        use super::functions::update_dns_records;
        update_dns_records(self, ip).await
    }

    fn validate_config(&self) -> Result<(), Self::Error> {
        if self.config.api_token.is_empty() || self.config.api_token == "your_api_token_here" {
            return Err(CloudflareError::InvalidApiToken(self.config.name.clone()));
        }
        if self.config.zone_id.is_empty() {
            return Err(CloudflareError::InvalidZoneId(self.config.name.clone()));
        }
        if self.config.subdomains.is_empty() {
            return Err(CloudflareError::NoSubdomains(self.config.name.clone()));
        }
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    fn get_name(&self) -> &str {
        &self.config.name
    }
}
