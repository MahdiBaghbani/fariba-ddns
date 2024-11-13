use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub log: Log,
    pub update: Update,
    pub cloudflare: Vec<Cloudflare>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Log {
    pub level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Update {
    pub interval: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Cloudflare {
    pub enabled: bool,
    pub name: String,
    pub zone_id: String,
    pub api_token: String,
    pub subdomains: Vec<CloudflareSubDomain>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CloudflareSubDomain {
    pub name: String,
}

/// Manages the application settings, allowing for loading and reloading configurations.
pub struct ConfigManager {
    pub settings: Arc<RwLock<Settings>>,
    pub config_path: PathBuf,
}
