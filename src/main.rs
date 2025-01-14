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
mod metrics;
mod providers;
mod settings;
mod utility;

// Project imports
use crate::metrics::{HealthChecker, MetricsManager};
use crate::providers::cloudflare::errors::CloudflareError;
use crate::providers::cloudflare::functions::{get_cloudflares, process_updates};
use crate::providers::cloudflare::types::Cloudflare;
use crate::providers::DnsProvider;
use crate::settings::types::{ConfigManager, Settings};

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

    // Create metrics and health checker
    let metrics = Arc::new(MetricsManager::new());
    let health = Arc::new(HealthChecker::new());

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
    run(config, metrics, health)
        .await
        .expect("Failed to run the application");
}

async fn run(
    config: Arc<ConfigManager>,
    metrics: Arc<MetricsManager>,
    health: Arc<HealthChecker>,
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

async fn get_public_ip_v4() -> Option<Ipv4Addr> {
    // attempt to get an IP address.
    public_ip::addr_v4().await
}

async fn get_public_ip_v6() -> Option<std::net::Ipv6Addr> {
    // attempt to get an IP address.
    public_ip::addr_v6().await
}
