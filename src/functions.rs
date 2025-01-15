// Standard library
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;

// 3rd party crates
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

// Project imports
use crate::providers::{
    self,
    cloudflare::{
        functions::{get_cloudflares, process_updates},
        types::Cloudflare,
    },
    DnsProvider,
};
use crate::settings::types::ConfigManager;
use crate::utility::ip_detector::types::{IpDetector, IpVersion};

/// Main application loop that handles IP monitoring and DNS updates.
///
/// This function:
/// - Monitors public IPv4 and IPv6 addresses with consensus validation
/// - Detects IP address changes reliably
/// - Updates DNS records when changes occur
/// - Handles network connectivity issues
/// - Respects configured update intervals and rate limits
/// - Implements graceful shutdown on signal
pub async fn run(
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

    // Run the first update immediately
    detect_and_update_ips(
        &ip_detector,
        &cloudflares,
        need_ipv4,
        need_ipv6,
        &mut previous_ipv4,
        &mut previous_ipv6,
        None,
        None,
    )
    .await;

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

            // Wait for the update interval
            _ = tokio::time::sleep(Duration::from_secs(update_interval)) => {
                detect_and_update_ips(
                    &ip_detector,
                    &cloudflares,
                    need_ipv4,
                    need_ipv6,
                    &mut previous_ipv4,
                    &mut previous_ipv6,
                    Some(ipv4_shutdown),
                    Some(ipv6_shutdown),
                ).await;
            }
        }
    }

    info!("Shutdown complete.");
    Ok(())
}

/// Performs a single IP detection cycle for both IPv4 and IPv6 if needed
async fn detect_and_update_ips(
    ip_detector: &IpDetector,
    cloudflares: &[Cloudflare],
    need_ipv4: bool,
    need_ipv6: bool,
    previous_ipv4: &mut Option<Ipv4Addr>,
    previous_ipv6: &mut Option<Ipv6Addr>,
    ipv4_shutdown: Option<broadcast::Receiver<()>>,
    ipv6_shutdown: Option<broadcast::Receiver<()>>,
) {
    debug!("Starting IP detection cycle");
    // Get the public IPv4 address with consensus if needed
    if need_ipv4 {
        debug!("Detecting IPv4 address");
        match ip_detector.detect_ip(IpVersion::V4).await {
            Ok(ip) => {
                if let IpAddr::V4(ipv4) = ip {
                    if Some(ipv4) != *previous_ipv4 {
                        info!("Public 🧩 IPv4 detected with consensus: {}", ipv4);
                        *previous_ipv4 = Some(ipv4);

                        // Process updates with pre-created subscription
                        if let Err(e) = process_updates(cloudflares, &ip, ipv4_shutdown).await {
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
                    if Some(ipv6) != *previous_ipv6 {
                        info!("Public 🧩 IPv6 detected with consensus: {}", ipv6);
                        *previous_ipv6 = Some(ipv6);

                        // Process updates with pre-created subscription
                        if let Err(e) = process_updates(cloudflares, &ip, ipv6_shutdown).await {
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
