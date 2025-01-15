// Standard library
use std::error::Error;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

// 3rd party crates
use tokio::sync::RwLockReadGuard;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{filter::LevelFilter, EnvFilter};

// Project modules
mod providers;
mod settings;
mod utility;

// Project imports
use crate::providers::cloudflare::functions::{get_cloudflares, process_updates};
use crate::providers::cloudflare::types::Cloudflare;
use crate::providers::DnsProvider;
use crate::settings::types::{ConfigManager, Settings};

/// Main entry point for the DDNS client.
/// This application monitors public IP addresses and updates DNS records
/// when changes are detected. It supports both IPv4 and IPv6 addresses.
/// 
/// Features:
/// - Automatic IP change detection
/// - Support for multiple DNS providers
/// - Concurrent DNS updates
/// - Rate limiting to respect API limits
/// - Configurable update intervals
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

    // Run the main application logic
    run(config)
        .await
        .expect("Failed to run the application");
}

/// Main application loop that handles IP monitoring and DNS updates.
/// 
/// This function:
/// - Monitors public IPv4 and IPv6 addresses
/// - Detects IP address changes
/// - Updates DNS records when changes occur
/// - Handles errors and retries
/// - Respects configured update intervals
async fn run(
    config: Arc<ConfigManager>,
) -> Result<(), Box<dyn Error>> {
    let update_interval: u64 = config.get_update_interval().await;

    info!("🕰️ Updating DNS records every {} seconds", update_interval);

    // Fetch settings and create Cloudflare instances
    let cloudflares: Vec<Cloudflare> = get_cloudflares(config).await?;

    let mut previous_ipv4: Option<Ipv4Addr> = None;
    let mut previous_ipv6: Option<std::net::Ipv6Addr> = None;

    loop {
        // Get the public IPv4 address
        match get_public_ip_v4().await {
            Some(ip) => {
                if Some(ip) != previous_ipv4 {
                    info!("Public 🧩 IPv4 detected: {}", ip);
                    previous_ipv4 = Some(ip);

                    // Process updates
                    if let Err(e) = process_updates(&cloudflares, &ip).await {
                        error!("Error updating IPv4 records: {}", e);
                    }
                } else {
                    debug!("🧩 IPv4 address unchanged");
                }
            }
            None => {
                warn!("🧩 IPv4 not detected");
            }
        }

        // Get the public IPv6 address
        match get_public_ip_v6().await {
            Some(ip) => {
                if Some(ip) != previous_ipv6 {
                    info!("Public 🧩 IPv6 detected: {}", ip);
                    previous_ipv6 = Some(ip);

                    // Process updates
                    for cloudflare in &cloudflares {
                        if cloudflare.config.enable_ipv6 {
                            if let Err(e) = cloudflare.update_dns_records_v6(&ip).await {
                                error!("Error updating IPv6 records: {}", e);
                            }
                        }
                    }
                } else {
                    debug!("🧩 IPv6 address unchanged");
                }
            }
            None => {
                debug!("🧩 IPv6 not detected");
            }
        }

        // Sleep for the update interval duration
        tokio::time::sleep(Duration::from_secs(update_interval)).await;
    }
}

/// Attempts to get the current public IPv4 address.
/// Uses the public_ip crate to query various IP detection services.
async fn get_public_ip_v4() -> Option<Ipv4Addr> {
    public_ip::addr_v4().await
}

/// Attempts to get the current public IPv6 address.
/// Uses the public_ip crate to query various IP detection services.
async fn get_public_ip_v6() -> Option<std::net::Ipv6Addr> {
    public_ip::addr_v6().await
}
