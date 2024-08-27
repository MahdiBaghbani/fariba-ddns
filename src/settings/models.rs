use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub log: Log,
    pub update: Update,
    pub cloudflare: Vec<Cloudflare>,
}

#[derive(Debug, Deserialize)]
pub struct Log {
    pub level: String,
}

#[derive(Debug, Deserialize)]
pub struct Update {
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct Cloudflare {
    pub enabled: bool,
    pub name: String,
    pub zone_id: String,
    pub api_token: String,
    pub subdomains: Vec<CloudflareSubDomain>,
}

#[derive(Debug, Deserialize)]
pub struct CloudflareSubDomain {
    pub name: String,
}
