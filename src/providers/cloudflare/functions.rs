// Standard library
use std::error::Error;
use std::net::Ipv4Addr;
use std::sync::Arc;

// 3rd party crates
use futures::{stream::FuturesUnordered, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{header, Client, StatusCode};
use serde_json::json;
use tokio::sync::RwLockReadGuard;
use tracing::{debug, error, info};

// Project modules
use crate::providers::DnsProvider;
use crate::settings::types::{ConfigManager, Settings};

use super::constants::CLOUDFLARE_API_BASE;
use super::errors::CloudflareError;
use super::types::{CfConfig, Cloudflare, DnsResponse, ZoneResponse};

/// Creates a reqwest client with the appropriate headers for Cloudflare API.
pub(super) fn create_reqwest_client(cloudflare: &CfConfig) -> Result<Client, CloudflareError> {
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

/// Updates DNS records for all configured subdomains.
pub async fn update_dns_records(
    cloudflare: &Cloudflare,
    ip: &Ipv4Addr,
) -> Result<(), CloudflareError> {
    // First verify the zone is active
    let zone_status = verify_zone_status(cloudflare).await?;
    if !zone_status.result.status.eq_ignore_ascii_case("active") {
        return Err(CloudflareError::InactiveZone(
            cloudflare.config.name.clone(),
            zone_status.result.status,
        ));
    }

    for subdomain in &cloudflare.config.subdomains {
        info!(
            zone = %cloudflare.config.name,
            subdomain = %subdomain.name,
            "Fetching DNS records"
        );

        let records = cloudflare
            .with_rate_limit(fetch_dns_records(cloudflare, &subdomain.name))
            .await?;

        println!("DNS records: {:?}", records);

        for record in records.result {
            if record.content != ip.to_string() {
                info!(
                    zone = %cloudflare.config.name,
                    subdomain = %subdomain.name,
                    "Updating DNS record from {} to {}",
                    record.content,
                    ip
                );

                match cloudflare
                    .with_rate_limit(update_record(cloudflare, &record.id, ip))
                    .await
                {
                    Ok(_) => {
                        info!(
                            zone = %cloudflare.config.name,
                            subdomain = %subdomain.name,
                            "Successfully updated DNS record to {}",
                            ip
                        );
                    }
                    Err(e) => {
                        error!(
                            zone = %cloudflare.config.name,
                            subdomain = %subdomain.name,
                            "Failed to update DNS record: {}",
                            e
                        );
                        return Err(e);
                    }
                }
            } else {
                debug!(
                    zone = %cloudflare.config.name,
                    subdomain = %subdomain.name,
                    "DNS record already set to {}",
                    ip
                );
            }
        }
    }
    Ok(())
}

/// Verifies that the zone is active
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

/// Fetches DNS records for a specific subdomain.
async fn fetch_dns_records(
    cloudflare: &Cloudflare,
    subdomain: &str,
) -> Result<DnsResponse, CloudflareError> {
    let url = format!(
        "{}/zones/{}/dns_records?type=A&name={}",
        CLOUDFLARE_API_BASE, cloudflare.config.zone_id, subdomain
    );

    let response =
        cloudflare
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CloudflareError::FetchFailed {
                zone: cloudflare.config.name.clone(),
                message: format!("Failed to fetch DNS records: {}", e),
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
        .json::<DnsResponse>()
        .await
        .map_err(|e| CloudflareError::FetchFailed {
            zone: cloudflare.config.name.clone(),
            message: format!("Failed to parse DNS response: {}", e),
        })
}

/// Updates a specific DNS record with a new IP address.
async fn update_record(
    cloudflare: &Cloudflare,
    record_id: &str,
    ip: &Ipv4Addr,
) -> Result<(), CloudflareError> {
    let url = format!(
        "{}/zones/{}/dns_records/{}",
        CLOUDFLARE_API_BASE, cloudflare.config.zone_id, record_id
    );

    let response = cloudflare
        .client
        .patch(&url)
        .json(&json!({
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

/// Gets all enabled Cloudflare instances from the configuration.
pub async fn get_cloudflares(
    config: Arc<ConfigManager>,
) -> Result<Vec<Cloudflare>, CloudflareError> {
    let settings: RwLockReadGuard<Settings> = config.get_settings().await;

    // Convert the vector of `CloudflareConfig` into a vector of `Cloudflare`.
    let cloudflares: Vec<Cloudflare> = settings
        .get_cloudflares()
        .into_iter()
        .filter_map(|config| match Cloudflare::new(config) {
            Ok(cf) if cf.is_enabled() => Some(cf),
            _ => None,
        })
        .collect();

    Ok(cloudflares)
}

/// Processes updates concurrently for multiple Cloudflare instances.
pub async fn process_updates(
    cloudflares: &[Cloudflare],
    ip: &Ipv4Addr,
) -> Result<(), Box<dyn Error>> {
    // Create a FuturesUnordered to hold our concurrent tasks.
    let mut futures = FuturesUnordered::new();

    // For each Cloudflare instance, spawn an async task to update DNS records.
    for cloudflare in cloudflares {
        info!(
            zone = %cloudflare.config.name,
            "Starting DNS update process"
        );
        // Push the future into the FuturesUnordered stream.
        futures.push(async move {
            // Call the method to update DNS records.
            cloudflare.update_dns_records(ip).await
        });
    }

    // Collect all results, processing them as they complete.
    while let Some(result) = futures.next().await {
        match result {
            Ok(_) => {
                debug!("Successfully updated Cloudflare records");
            }
            Err(e) => {
                error!("Error updating Cloudflare records: {}", e);
                return Err(Box::new(e));
            }
        }
    }

    Ok(())
}
