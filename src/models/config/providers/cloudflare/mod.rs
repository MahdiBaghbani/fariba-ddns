use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Cloudflare {
    pub enabled: bool,
    pub domains: Vec<Domain>,
}

#[derive(Serialize, Deserialize)]
pub struct Domain {
    pub authentication: Authentication,
    pub zone_id: String,
    pub subdomains: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Authentication {
    pub api_token: String,
    pub api_key: ApiKey,
}

#[derive(Serialize, Deserialize)]
pub struct ApiKey {
    pub api_key: String,
    pub account_email: String,
}
