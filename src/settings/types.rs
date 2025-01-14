// Standard library
use std::path::PathBuf;
use std::sync::Arc;

// 3rd party crates
use serde::Deserialize;
use tokio::sync::RwLock;

// Project imports
use crate::providers::cloudflare::types::CfConfig;

#[derive(Debug, Deserialize, Clone)]
pub struct Log {
    #[serde(default = "default_log_level")]
    pub level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Update {
    #[serde(default = "default_update_interval")]
    pub interval: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Cache {
    #[serde(default = "default_cache_ttl")]
    pub ttl: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub log: Log,
    pub update: Update,
    pub cache: Cache,

    #[serde(default)]
    pub cloudflare: Vec<CfConfig>,
}

fn default_update_interval() -> u64 {
    300 // 5 minutes
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_cache_ttl() -> u64 {
    60 // 1 minute in seconds
}

/// Manages the application settings, allowing for loading and reloading configurations.
pub struct ConfigManager {
    pub settings: Arc<RwLock<Settings>>,
    pub _config_path: PathBuf,
}
