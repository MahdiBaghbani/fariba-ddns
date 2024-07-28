use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Arvancloud {
    pub enabled: bool,
    pub domains: Vec<Domain>,
}

#[derive(Serialize, Deserialize)]
pub struct Domain {
    pub authentication: Authentication,
    pub domain: String,
    pub subdomains: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Authentication {
    pub api_token: String,
}
