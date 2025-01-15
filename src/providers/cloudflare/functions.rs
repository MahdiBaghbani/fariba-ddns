// Standard library
use std::error::Error;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

// 3rd party crates
use futures::{stream::FuturesUnordered, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{header, Client, StatusCode};
use serde_json::json;
use tokio::sync::{broadcast, RwLockReadGuard};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

// Project modules
use crate::providers::DnsProvider;
use crate::settings::types::{ConfigManager, Settings};

// Current module imports
use super::constants::CLOUDFLARE_API_BASE;
use super::errors::CloudflareError;
use super::types::{CfConfig, Cloudflare, DnsResponse, ZoneResponse};

/// Creates a reqwest client with the appropriate headers for Cloudflare API.
/// This includes setting up authentication headers and other necessary configuration.
pub fn create_reqwest_client(cloudflare: &CfConfig) -> Result<Client, CloudflareError> {
    if cloudflare.api_token.is_empty() || cloudflare.api_token == "your_api_token_here" {
        error!(
            zone = %cloudflare.name,
            "API token is not set or invalid for '{}'",
            cloudflare.name
        );
        return Err(CloudflareError::InvalidApiToken(cloudflare.name.clone()));
    }

    // Create headers.
    let mut headers: HeaderMap = HeaderMap::new();

    // Mark security-sensitive headers with `set_sensitive`.
    let bearer_token: String = format!("Bearer {}", &cloudflare.api_token);
    let mut auth_value: HeaderValue = HeaderValue::from_str(&bearer_token).map_err(|e| {
        error!(
            zone = %cloudflare.name,
            "Invalid API token format: {}",
            e
        );
        CloudflareError::InvalidHeaderValue(e)
    })?;
    auth_value.set_sensitive(true);
    headers.insert(header::AUTHORIZATION, auth_value);

    // Build the client.
    let client: Client = Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| {
            error!(
                zone = %cloudflare.name,
                "Failed to build HTTP client: {}",
                e
            );
            CloudflareError::HttpClientBuild(e)
        })?;

    Ok(client)
}

/// Gets all enabled Cloudflare instances from the configuration.
/// This function creates Cloudflare clients for each enabled configuration,
/// initializing them with the appropriate settings.
pub async fn get_cloudflares(
    config: Arc<ConfigManager>,
) -> Result<Vec<Cloudflare>, Box<dyn Error>> {
    let settings: RwLockReadGuard<Settings> = config.settings.read().await;

    let mut cloudflares = Vec::new();
    for cf_config in settings.cloudflare.iter() {
        if cf_config.enabled {
            match Cloudflare::new(cf_config.clone()) {
                Ok(cloudflare) => cloudflares.push(cloudflare),
                Err(e) => error!("Failed to create Cloudflare instance: {}", e),
            }
        }
    }
    Ok(cloudflares)
}

/// Processes updates concurrently for multiple Cloudflare instances.
/// This function handles updating DNS records for multiple domains in parallel,
/// using a FuturesUnordered to manage concurrent updates efficiently.
/// Now includes graceful shutdown handling.
pub async fn process_updates(
    cloudflares: &[Cloudflare],
    ip: &IpAddr,
    shutdown_rx: Option<broadcast::Receiver<()>>,
) -> Result<(), Box<dyn Error>> {
    // Create a FuturesUnordered to hold our concurrent tasks.
    let futures = FuturesUnordered::new();

    // For each Cloudflare instance, spawn an async task to update DNS records.
    for cloudflare in cloudflares {
        info!(
            zone = %cloudflare.config.name,
            "Starting DNS update process"
        );
        // Push the future into the FuturesUnordered stream.
        let cloudflare = cloudflare.clone();
        let ip = *ip;
        futures.push(async move {
            // Call the method to update DNS records.
            cloudflare.update_dns_records_ip(&ip).await
        });
    }

    // Set a timeout for the entire update process
    let update_timeout = Duration::from_secs(30);

    // Process updates with timeout and shutdown handling
    match timeout(
        update_timeout,
        process_updates_with_shutdown(futures, shutdown_rx),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            error!(
                "DNS updates timed out after {} seconds",
                update_timeout.as_secs()
            );
            Err(Box::new(CloudflareError::UpdateTimeout))
        }
    }
}

/// Helper function to process updates with shutdown handling
async fn process_updates_with_shutdown(
    mut futures: FuturesUnordered<impl std::future::Future<Output = Result<(), CloudflareError>>>,
    mut shutdown_rx: Option<broadcast::Receiver<()>>,
) -> Result<(), Box<dyn Error>> {
    let mut update_count = 0;
    let mut last_error = None;

    loop {
        tokio::select! {
            // Handle shutdown signal if provided
            shutdown = async {
                if let Some(rx) = &mut shutdown_rx {
                    rx.recv().await
                } else {
                    Ok(())
                }
            } => {
                match shutdown {
                    Ok(_) => {
                        info!("Received shutdown signal during DNS updates, waiting for in-progress updates...");
                        // Allow a short time for in-progress updates to complete
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        break;
                    }
                    Err(e) => {
                        warn!("Shutdown receiver error: {}", e);
                        // Continue processing if there's a receiver error
                        continue;
                    }
                }
            }
            // Process next update
            Some(result) = futures.next() => {
                match result {
                    Ok(_) => {
                        update_count += 1;
                        debug!("Successfully completed DNS update {}", update_count);
                    }
                    Err(e) => {
                        error!("Error updating DNS records: {}", e);
                        last_error = Some(e);
                    }
                }

                // Check if all updates are complete
                if futures.is_empty() {
                    break;
                }
            }
            // All futures completed
            else => break,
        }
    }

    // Report results
    if update_count > 0 {
        info!("Completed {} DNS updates", update_count);
        Ok(())
    } else if let Some(e) = last_error {
        Err(Box::new(e))
    } else {
        Ok(())
    }
}

/// Fetches DNS records for a specific domain.
/// This function retrieves the current A or AAAA records for a domain from Cloudflare's API.
/// It includes error handling for various API response scenarios.
async fn fetch_dns_records(
    cloudflare: &Cloudflare,
    domain: &str,
    record_type: &str,
) -> Result<DnsResponse, CloudflareError> {
    let url = format!(
        "{}/zones/{}/dns_records?type={}&name={}",
        CLOUDFLARE_API_BASE, cloudflare.config.zone_id, record_type, domain
    );

    debug!(
        zone = %cloudflare.config.name,
        domain = %domain,
        url = %url,
        "Sending DNS records request"
    );

    let response =
        tokio::time::timeout(Duration::from_secs(10), cloudflare.client.get(&url).send())
            .await
            .map_err(|_| CloudflareError::Timeout {
                zone: cloudflare.config.name.clone(),
                message: "DNS record fetch request timed out".to_string(),
            })??;

    let status = response.status();
    match status {
        StatusCode::OK => {
            let response_text =
                response
                    .text()
                    .await
                    .map_err(|e| CloudflareError::FetchFailed {
                        zone: cloudflare.config.name.clone(),
                        message: format!("Failed to read response body: {}", e),
                    })?;

            debug!(
                zone = %cloudflare.config.name,
                domain = %domain,
                response = %response_text,
                "Received DNS records response"
            );

            serde_json::from_str(&response_text).map_err(|e| CloudflareError::FetchFailed {
                zone: cloudflare.config.name.clone(),
                message: format!("Failed to parse response: {}", e),
            })
        }
        StatusCode::UNAUTHORIZED => Err(CloudflareError::InvalidApiToken(
            cloudflare.config.name.clone(),
        )),
        _ => Err(CloudflareError::FetchFailed {
            zone: cloudflare.config.name.clone(),
            message: format!("HTTP {}", status),
        }),
    }
}

/// Updates DNS records for all configured subdomains.
/// This function:
/// - Verifies the zone is active
/// - Processes each subdomain
/// - Handles retries on failure
/// - Provides detailed logging of the update process
pub async fn update_dns_records(
    cloudflare: &Cloudflare,
    ip: &IpAddr,
) -> Result<(), CloudflareError> {
    // First verify the zone is active
    let zone_status = verify_zone_status(cloudflare).await?;
    if !zone_status.result.status.eq_ignore_ascii_case("active") {
        return Err(CloudflareError::InactiveZone(
            cloudflare.config.name.clone(),
            zone_status.result.status,
        ));
    }

    let mut last_error: Option<CloudflareError> = None;
    let mut update_count = 0;
    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 3;

    let record_type = match ip {
        IpAddr::V4(_) => "A",
        IpAddr::V6(_) => "AAAA",
    };

    for subdomain in &cloudflare.config.subdomains {
        // Skip if this IP version is not enabled for this subdomain
        match (ip, &subdomain.ip_version) {
            (IpAddr::V4(_), super::types::IpVersion::V6)
            | (IpAddr::V6(_), super::types::IpVersion::V4) => {
                debug!(
                    zone = %cloudflare.config.name,
                    subdomain = %subdomain.name,
                    ip_type = %record_type,
                    "Skipping DNS update - IP version not enabled for subdomain"
                );
                continue;
            }
            _ => {}
        }

        // Construct the full domain name for logging
        let full_domain = if subdomain.name.is_empty() {
            cloudflare.config.name.clone()
        } else {
            format!("{}.{}", subdomain.name, cloudflare.config.name)
        };

        info!(
            zone = %cloudflare.config.name,
            domain = %full_domain,
            "Processing DNS records"
        );

        'retry: loop {
            match process_domain_record(cloudflare, &full_domain, ip, record_type).await {
                Ok(_) => {
                    update_count += 1;
                    break 'retry;
                }
                Err(e) => {
                    if retry_count < MAX_RETRIES {
                        retry_count += 1;
                        warn!(
                            zone = %cloudflare.config.name,
                            domain = %full_domain,
                            error = %e,
                            retry = retry_count,
                            "Retrying after error"
                        );
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                    error!(
                        zone = %cloudflare.config.name,
                        domain = %full_domain,
                        error = %e,
                        "Failed after {} retries",
                        MAX_RETRIES
                    );
                    last_error = Some(e);
                    break 'retry;
                }
            }
        }
    }

    // Log summary
    if update_count > 0 {
        info!(
            zone = %cloudflare.config.name,
            count = update_count,
            "Successfully processed {} DNS records",
            update_count
        );
    }

    if let Some(error) = last_error {
        Err(error)
    } else {
        Ok(())
    }
}

/// Process a single domain record - fetch, create if missing, or update if needed.
/// This function handles the core logic for managing a single domain's DNS records:
/// - Fetches current records
/// - Creates new records if none exist
/// - Updates records if IP has changed
/// - Handles rate limiting through the with_rate_limit wrapper
async fn process_domain_record(
    cloudflare: &Cloudflare,
    full_domain: &str,
    ip: &IpAddr,
    record_type: &str,
) -> Result<(), CloudflareError> {
    let records = cloudflare
        .with_rate_limit(fetch_dns_records(cloudflare, full_domain, record_type))
        .await?;

    if records.result.is_empty() {
        warn!(
            zone = %cloudflare.config.name,
            domain = %full_domain,
            "No DNS records found, attempting to create"
        );
        return cloudflare
            .with_rate_limit(create_dns_record(cloudflare, full_domain, ip, record_type))
            .await;
    }

    for record in records.result {
        if record.content != ip.to_string() {
            info!(
                zone = %cloudflare.config.name,
                domain = %full_domain,
                "Updating DNS record from {} to {}",
                record.content,
                ip
            );

            match cloudflare
                .with_rate_limit(update_record(cloudflare, &record.id, ip, record_type))
                .await
            {
                Ok(_) => {
                    info!(
                        zone = %cloudflare.config.name,
                        domain = %full_domain,
                        "Successfully updated DNS record to {}",
                        ip
                    );
                }
                Err(e) => {
                    error!(
                        zone = %cloudflare.config.name,
                        domain = %full_domain,
                        "Failed to update DNS record: {}",
                        e
                    );
                    return Err(e);
                }
            }
        } else {
            debug!(
                zone = %cloudflare.config.name,
                domain = %full_domain,
                "DNS record already set to {}",
                ip
            );
        }
    }

    Ok(())
}

/// Creates a new DNS record with the specified IP address.
/// This function handles the creation of new A or AAAA records in Cloudflare,
/// including proper error handling and validation.
async fn create_dns_record(
    cloudflare: &Cloudflare,
    domain: &str,
    ip: &IpAddr,
    record_type: &str,
) -> Result<(), CloudflareError> {
    info!(
        zone = %cloudflare.config.name,
        domain = %domain,
        "Creating new DNS record with IP {}",
        ip
    );

    let url = format!(
        "{}/zones/{}/dns_records",
        CLOUDFLARE_API_BASE, cloudflare.config.zone_id
    );

    let response = cloudflare
        .client
        .post(&url)
        .json(&json!({
            "type": record_type,
            "name": domain,
            "content": ip.to_string(),
            "proxied": true,
            "ttl": 1, // Auto TTL
        }))
        .send()
        .await
        .map_err(|e| CloudflareError::CreateFailed {
            zone: cloudflare.config.name.clone(),
            domain: domain.to_string(),
            message: format!("Failed to send create request: {}", e),
        })?;

    let status = response.status();
    if status == StatusCode::UNAUTHORIZED {
        return Err(CloudflareError::InvalidApiToken(
            cloudflare.config.name.clone(),
        ));
    }

    if !status.is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(CloudflareError::CreateFailed {
            zone: cloudflare.config.name.clone(),
            domain: domain.to_string(),
            message: format!("HTTP {} - {}", status, error_body),
        });
    }

    info!(
        zone = %cloudflare.config.name,
        domain = %domain,
        "Successfully created DNS record"
    );
    Ok(())
}

/// Updates a specific DNS record with a new IP address.
/// This function updates an existing A or AAAA record with a new IP address,
/// handling all necessary API interactions and error cases.
async fn update_record(
    cloudflare: &Cloudflare,
    record_id: &str,
    ip: &IpAddr,
    record_type: &str,
) -> Result<(), CloudflareError> {
    let url = format!(
        "{}/zones/{}/dns_records/{}",
        CLOUDFLARE_API_BASE, cloudflare.config.zone_id, record_id
    );

    let response = cloudflare
        .client
        .patch(&url)
        .json(&json!({
            "type": record_type,
            "content": ip.to_string(),
            "proxied": true
        }))
        .send()
        .await
        .map_err(|e| CloudflareError::UpdateFailed {
            zone: cloudflare.config.name.clone(),
            message: format!("Failed to send update request: {}", e),
        })?;

    let status = response.status();
    if status == StatusCode::UNAUTHORIZED {
        return Err(CloudflareError::InvalidApiToken(
            cloudflare.config.name.clone(),
        ));
    }

    if !status.is_success() {
        return Err(CloudflareError::UpdateFailed {
            zone: cloudflare.config.name.clone(),
            message: format!("HTTP {}", status),
        });
    }

    Ok(())
}

/// Verifies that the zone is active.
/// This function checks if the Cloudflare zone is active and available
/// for DNS record management.
async fn verify_zone_status(cloudflare: &Cloudflare) -> Result<ZoneResponse, CloudflareError> {
    let url = format!(
        "{}/zones/{}",
        CLOUDFLARE_API_BASE, cloudflare.config.zone_id
    );

    let response =
        cloudflare
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CloudflareError::FetchFailed {
                zone: cloudflare.config.name.clone(),
                message: format!("Failed to fetch zone status: {}", e),
            })?;

    let status = response.status();
    if status == StatusCode::UNAUTHORIZED {
        return Err(CloudflareError::InvalidApiToken(
            cloudflare.config.name.clone(),
        ));
    }

    if !status.is_success() {
        return Err(CloudflareError::FetchFailed {
            zone: cloudflare.config.name.clone(),
            message: format!("HTTP {}", status),
        });
    }

    response
        .json::<ZoneResponse>()
        .await
        .map_err(|e| CloudflareError::FetchFailed {
            zone: cloudflare.config.name.clone(),
            message: format!("Failed to parse zone response: {}", e),
        })
}
