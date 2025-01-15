// Standard library
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;

// 3rd party crates
use tokio::signal::ctrl_c;
use tokio::sync::{broadcast, RwLockReadGuard};
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
use crate::utility::ip_detector::types::{IpDetector, IpVersion};

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

/// Main application loop that handles IP monitoring and DNS updates.
///
/// This function:
/// - Monitors public IPv4 and IPv6 addresses with consensus validation
/// - Detects IP address changes reliably
/// - Updates DNS records when changes occur
/// - Handles network connectivity issues
/// - Respects configured update intervals and rate limits
/// - Implements graceful shutdown on signal
async fn run(
    config: Arc<ConfigManager>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<(), Box<dyn Error>> {
    let settings = config.settings.read().await;
    let update_interval: u64 = settings.update.interval;
    info!("🕰️ Updating DNS records every {} seconds", update_interval);

    // Initialize IP detector with configuration
    let ip_detector = IpDetector::new(settings.ip_detection.clone());

    // Fetch settings and create Cloudflare instances
    let cloudflares: Vec<Cloudflare> = get_cloudflares(Arc::clone(&config)).await?;

    // Determine which IP versions we need to detect based on subdomain configurations
    let mut need_ipv4 = false;
    let mut need_ipv6 = false;
    for cf in &cloudflares {
        if !cf.is_enabled() {
            continue;
        }
        for subdomain in &cf.config.subdomains {
            match subdomain.ip_version {
                providers::cloudflare::types::IpVersion::V4 => need_ipv4 = true,
                providers::cloudflare::types::IpVersion::V6 => need_ipv6 = true,
                providers::cloudflare::types::IpVersion::Both => {
                    need_ipv4 = true;
                    need_ipv6 = true;
                }
            }
            if need_ipv4 && need_ipv6 {
                break;
            }
        }
        if need_ipv4 && need_ipv6 {
            break;
        }
    }

    info!(
        "IP detection configuration - IPv4: {}, IPv6: {}",
        need_ipv4, need_ipv6
    );

    // Drop the settings lock
    drop(settings);

    let mut previous_ipv4: Option<Ipv4Addr> = None;
    let mut previous_ipv6: Option<Ipv6Addr> = None;

    loop {
        // Create subscriptions for DNS updates before entering select!
        let ipv4_shutdown = shutdown_rx.resubscribe();
        let ipv6_shutdown = shutdown_rx.resubscribe();

        tokio::select! {
            // Handle shutdown signal
            Ok(_) = shutdown_rx.recv() => {
                info!("Received shutdown signal, waiting for in-progress updates...");
                // Allow a short time for in-progress updates to complete
                tokio::time::sleep(Duration::from_secs(5)).await;
                break;
            }

            // Handle periodic updates
            _ = tokio::time::sleep(Duration::from_secs(update_interval)) => {
                debug!("Starting IP detection cycle");
                // Get the public IPv4 address with consensus if needed
                if need_ipv4 {
                    debug!("Detecting IPv4 address");
                    match ip_detector.detect_ip(IpVersion::V4).await {
                        Ok(ip) => {
                            if let IpAddr::V4(ipv4) = ip {
                                if Some(ipv4) != previous_ipv4 {
                                    info!("Public 🧩 IPv4 detected with consensus: {}", ipv4);
                                    previous_ipv4 = Some(ipv4);

                                    // Process updates with pre-created subscription
                                    if let Err(e) = process_updates(&cloudflares, &ip, Some(ipv4_shutdown)).await {
                                        error!("Error updating IPv4 records: {}", e);
                                    }
                                } else {
                                    debug!("🧩 IPv4 address unchanged");
                                }
                            }
                        }
                        Err(e) => {
                            // Log IPv4 errors as warnings since IPv4 is critical
                            warn!("🧩 IPv4 detection failed: {}", e);
                        }
                    }
                } else {
                    debug!("Skipping IPv4 detection - not needed by any subdomain");
                }

                // Get the public IPv6 address with consensus if needed
                if need_ipv6 {
                    debug!("Detecting IPv6 address");
                    match ip_detector.detect_ip(IpVersion::V6).await {
                        Ok(ip) => {
                            if let IpAddr::V6(ipv6) = ip {
                                if Some(ipv6) != previous_ipv6 {
                                    info!("Public 🧩 IPv6 detected with consensus: {}", ipv6);
                                    previous_ipv6 = Some(ipv6);

                                    // Process updates with pre-created subscription
                                    if let Err(e) = process_updates(&cloudflares, &ip, Some(ipv6_shutdown)).await {
                                        error!("Error updating IPv6 records: {}", e);
                                    }
                                } else {
                                    debug!("🧩 IPv6 address unchanged");
                                }
                            }
                        }
                        Err(e) => {
                            // Log IPv6 errors as debug since IPv6 is optional
                            debug!("🧩 IPv6 detection failed: {}", e);
                        }
                    }
                } else {
                    debug!("Skipping IPv6 detection - not needed by any subdomain");
                }
            }
        }
    }

    info!("Shutdown complete.");
    Ok(())
}
