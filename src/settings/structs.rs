// Standard library
use std::path::PathBuf;
use std::sync::Arc;

// 3rd party crates
use serde::Deserialize;
use tokio::sync::RwLock;

// Project imports
use crate::providers::cloudflare::structs::CfConfig;

#[derive(Debug, Deserialize, Clone)]
pub struct Log {
    pub level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Update {
    pub interval: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub log: Log,
    pub update: Update,
    pub cloudflare: Vec<CfConfig>,
}

/// Manages the application settings, allowing for loading and reloading configurations.
pub struct ConfigManager {
    pub settings: Arc<RwLock<Settings>>,
    pub _config_path: PathBuf,
}
