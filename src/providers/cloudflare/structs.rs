// 3rd party crates
use reqwest::Client;
use serde::Deserialize;

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
