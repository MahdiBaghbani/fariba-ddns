// Standard library
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs};

// 3rd party crates
use config::{Config, ConfigError, File};
use log::{error, info, LevelFilter};
use tokio::sync::RwLock;

// Project imports
use crate::providers::cloudflare::types::CfConfig;

// Current module imports
use super::constants::DEFAULT_CONFIG;
use super::types::{ConfigManager, Settings};

impl Settings {
    pub fn get_log_level(&self) -> String {
        self.log.level.to_lowercase()
    }

    pub fn get_update_interval(&self) -> u64 {
        self.update.interval
    }

    pub fn get_cloudflares(&self) -> Vec<CfConfig> {
        self.cloudflare.clone()
    }
}

impl ConfigManager {
    /// Creates a new `ConfigManager` instance by loading the configuration.
    pub async fn new() -> Result<Self, ConfigError> {
        let config_path: PathBuf = Self::get_config_path()?;
        Self::ensure_config_file_exists(&config_path)?;

        let settings: Settings = Self::load_settings(&config_path)?;

        let manager = ConfigManager {
            settings: Arc::new(RwLock::new(settings)),
            _config_path: config_path,
        };

        manager.adjust_logging_level().await;

        Ok(manager)
    }

    /// Determines the configuration file path.
    fn get_config_path() -> Result<PathBuf, ConfigError> {
        if let Ok(path) = env::var("FDDNS_CONFIG_PATH") {
            Ok(PathBuf::from(path))
        } else if let Some(config_dir) = dirs::config_dir() {
            Ok(config_dir.join("fddns").join("config.toml"))
        } else {
            let msg: &str = "Could not determine the configuration directory";
            error!("{}", msg);
            Err(ConfigError::Message(msg.into()))
        }
    }

    /// Ensures that the configuration file exists, creating it if necessary.
    fn ensure_config_file_exists(config_path: &Path) -> Result<(), ConfigError> {
        if !config_path.exists() {
            if let Some(parent_dir) = config_path.parent() {
                fs::create_dir_all(parent_dir).map_err(|e| {
                    let msg: String = format!("Failed to create configuration directory: {}", e);
                    error!("{}", msg);
                    ConfigError::Message(msg)
                })?;
            }
            fs::write(config_path, DEFAULT_CONFIG).map_err(|e| {
                let msg: String = format!("Failed to create default configuration file: {}", e);
                error!("{}", msg);
                ConfigError::Message(msg)
            })?;
            info!("Default configuration file created at: {:?}", config_path);
        }
        Ok(())
    }

    /// Loads the settings from the configuration file and environment variables.
    fn load_settings(config_path: &Path) -> Result<Settings, ConfigError> {
        let config_file: &str = config_path.to_str().ok_or_else(|| {
            let msg: &str = "Configuration file path contains invalid UTF-8 characters";
            error!("{}", msg);
            ConfigError::Message(msg.into())
        })?;

        let settings: Config = Config::builder()
            .add_source(File::with_name(config_file))
            .build()?;

        settings.try_deserialize()
    }

    /// Reloads the configuration from the file.
    pub async fn _reload(&self) -> Result<(), ConfigError> {
        let new_settings: Settings = Self::load_settings(&self._config_path)?;
        *self.settings.write().await = new_settings;
        self.adjust_logging_level().await;
        info!("Configuration reloaded from {:?}", self._config_path);
        Ok(())
    }

    /// Adjusts the logging level based on the configuration.
    async fn adjust_logging_level(&self) {
        let level: String = self.get_log_level().await;
        let level_filter: LevelFilter = match level.as_str() {
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Info,
        };
        log::set_max_level(level_filter);
    }

    /// Provides a read-locked reference to the current settings.
    pub async fn get_settings(&self) -> tokio::sync::RwLockReadGuard<'_, Settings> {
        self.settings.read().await
    }

    /// Provides an `Arc` to the settings `RwLock`.
    pub fn _get_settings_arc(&self) -> Arc<RwLock<Settings>> {
        Arc::clone(&self.settings)
    }

    pub async fn get_log_level(&self) -> String {
        self.settings.read().await.get_log_level()
    }

    pub async fn get_update_interval(&self) -> u64 {
        self.settings.read().await.get_update_interval()
    }
}
