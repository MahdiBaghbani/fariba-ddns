//! Fariba DDNS Client
//!
//! A flexible Dynamic DNS client that supports multiple DNS providers.
//! This client automatically updates DNS records when your IP address changes,
//! making it ideal for home-lab and self-hosted services.
//!
//! # Features
//!
//! - Multiple DNS provider support (Cloudflare, ArvanCloud, etc.)
//! - Automatic IP detection using various services
//! - Configurable update intervals
//! - Detailed logging and error reporting
//!
//! # Example
//!
//! ```no_run
//! # async fn run() {
//! use fariba_ddns::ConfigManager;
//!
//! let config = ConfigManager::from_file(".settings.toml").await?;
//! fariba_ddns::run(config).await?;
//! # }
//! ```

// Standard library
use std::sync::Arc;

// 3rd party crates
use tokio::signal::ctrl_c;
use tokio::sync::broadcast;
use tracing::{error, info};
use tracing_subscriber::{filter::LevelFilter, EnvFilter};

// Project modules
mod functions;
mod providers;
mod settings;
mod utility;

// Project imports
use crate::functions::run;
use crate::settings::types::ConfigManager;

/// Main entry point for the DDNS client.
/// This application monitors public IP addresses and updates DNS records
/// when changes are detected. It supports both IPv4 and IPv6 addresses.
///
/// Features:
/// - Automatic IP change detection with consensus validation
/// - Support for multiple DNS providers
/// - Concurrent DNS updates
/// - Rate limiting to respect API limits
/// - Configurable update intervals
/// - Network connectivity monitoring
/// - Detailed logging
#[tokio::main]
async fn main() {
    // loads the .env file from the current directory or parents.
    dotenvy::dotenv_override().ok();

    // Create ConfigManager and wrap it in Arc
    let config: Arc<ConfigManager> = Arc::new(
        ConfigManager::new()
            .await
            .expect("Failed to initialize configuration"),
    );

    // setup logging.
    let log_level: String = config.get_log_level().await;

    let filter: EnvFilter = EnvFilter::builder()
        .with_default_directive(LevelFilter::ERROR.into())
        .parse_lossy(log_level)
        .add_directive("hyper_util=error".parse().unwrap())
        .add_directive("reqwest=error".parse().unwrap())
        .add_directive("trust_dns_proto=error".parse().unwrap())
        .add_directive("hyper_system_resolver=error".parse().unwrap())
        .add_directive("hyper=error".parse().unwrap());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_level(true)
        .init();

    info!("⚙️ Settings have been loaded.");

    // Create a broadcast channel for shutdown signal
    let (shutdown_tx, _) = broadcast::channel(1);
    let shutdown_tx_clone = shutdown_tx.clone();

    // Handle Ctrl+C
    tokio::spawn(async move {
        if let Err(e) = ctrl_c().await {
            error!("Failed to listen for Ctrl+C: {}", e);
            return;
        }
        info!("Received shutdown signal, initiating graceful shutdown...");
        let _ = shutdown_tx_clone.send(());
    });

    // Run the main application logic with shutdown signal
    if let Err(e) = run(config, shutdown_tx.subscribe()).await {
        error!("Application error: {}", e);
    }

    info!("Shutdown complete.");
}
