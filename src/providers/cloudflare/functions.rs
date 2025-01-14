// Standard library
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Instant;

// 3rd party crates
use futures::{stream::FuturesUnordered, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{header, Client, StatusCode};
use serde_json::json;
use tokio::sync::RwLockReadGuard;
use tracing::{debug, error, info, warn};

// Project modules
use crate::metrics::{HealthChecker, MetricsManager};
use crate::providers::DnsProvider;
use crate::settings::types::{ConfigManager, Settings};
use crate::utility::{CachedRecord, SharedDnsCache};

use super::constants::CLOUDFLARE_API_BASE;
use super::errors::CloudflareError;
use super::types::{CfConfig, Cloudflare, DnsResponse, DnsResponseResult, ZoneResponse};

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

/// Creates a new DNS record
async fn create_dns_record(
    cloudflare: &Cloudflare,
    domain: &str,
    ip: &Ipv4Addr,
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
            "type": "A",
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

    let mut last_error: Option<CloudflareError> = None;
    let mut update_count = 0;
    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 3;

    for subdomain in &cloudflare.config.subdomains {
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
            match process_domain_record(cloudflare, &full_domain, ip).await {
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
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
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

/// Process a single domain record - fetch, create if missing, or update if needed
async fn process_domain_record(
    cloudflare: &Cloudflare,
    full_domain: &str,
    ip: &Ipv4Addr,
) -> Result<(), CloudflareError> {
    let records = cloudflare
        .with_rate_limit(fetch_dns_records(cloudflare, full_domain))
        .await?;

    if records.result.is_empty() {
        warn!(
            zone = %cloudflare.config.name,
            domain = %full_domain,
            "No DNS records found, attempting to create"
        );
        return cloudflare
            .with_rate_limit(create_dns_record(cloudflare, full_domain, ip))
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
                .with_rate_limit(update_record(cloudflare, &record.id, ip))
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

/// Fetches DNS records for a specific domain.
async fn fetch_dns_records(
    cloudflare: &Cloudflare,
    domain: &str,
) -> Result<DnsResponse, CloudflareError> {
    // Check cache first
    if let Some(cached) = cloudflare.cache.get(domain).await {
        debug!(
            zone = %cloudflare.config.name,
            domain = %domain,
            "Using cached DNS record"
        );
        return Ok(DnsResponse {
            result: vec![DnsResponseResult {
                id: cached.record_id,
                content: cached.ip.to_string(),
                r#type: "".to_string(),
            }],
        });
    }

    let url = format!(
        "{}/zones/{}/dns_records?type=A&name={}",
        CLOUDFLARE_API_BASE, cloudflare.config.zone_id, domain
    );

    debug!(
        zone = %cloudflare.config.name,
        domain = %domain,
        url = %url,
        "Sending DNS records request"
    );

    let response = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        cloudflare.client.get(&url).send(),
    )
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

            let dns_response: DnsResponse =
                serde_json::from_str(&response_text).map_err(|e| CloudflareError::FetchFailed {
                    zone: cloudflare.config.name.clone(),
                    message: format!("Failed to parse response: {}", e),
                })?;

            // Cache the result if we got records
            if let Some(record) = dns_response.result.first() {
                if let Ok(ip) = record.content.parse::<IpAddr>() {
                    cloudflare
                        .cache
                        .insert(
                            domain.to_string(),
                            CachedRecord {
                                ip,
                                record_id: record.id.clone(),
                                provider: "cloudflare".to_string(),
                                timestamp: Instant::now(),
                            },
                        )
                        .await;
                }
            }

            Ok(dns_response)
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

    // Invalidate cache for this domain
    if let Some(domain) = get_domain_for_record_id(cloudflare, record_id).await {
        cloudflare.cache.invalidate(&domain).await;
    }

    Ok(())
}

/// Helper function to get domain from record_id
async fn get_domain_for_record_id(cloudflare: &Cloudflare, record_id: &str) -> Option<String> {
    for subdomain in &cloudflare.config.subdomains {
        let full_domain = if subdomain.name.is_empty() {
            cloudflare.config.name.clone()
        } else {
            format!("{}.{}", subdomain.name, cloudflare.config.name)
        };

        if let Ok(response) = fetch_dns_records(cloudflare, &full_domain).await {
            if response.result.iter().any(|r| r.id == record_id) {
                return Some(full_domain);
            }
        }
    }
    None
}

/// Gets all enabled Cloudflare instances with shared monitoring
pub async fn get_cloudflares_with_monitoring(
    config: Arc<ConfigManager>,
    metrics: Arc<MetricsManager>,
    health: Arc<HealthChecker>,
) -> Result<Vec<Cloudflare>, Box<dyn Error>> {
    let settings: RwLockReadGuard<Settings> = config.settings.read().await;
    let cache = SharedDnsCache::new(settings.cache.ttl);

    let mut cloudflares = Vec::new();
    for cf_config in settings.cloudflare.iter() {
        if cf_config.enabled {
            match Cloudflare::new_with_monitoring(
                cf_config.clone(),
                Arc::clone(&metrics),
                Arc::clone(&health),
                cache.clone(),
            ) {
                Ok(cloudflare) => cloudflares.push(cloudflare),
                Err(e) => error!("Failed to create Cloudflare instance: {}", e),
            }
        }
    }
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

/// Updates DNS records for all configured subdomains with IPv6.
pub async fn update_dns_records_v6(
    cloudflare: &Cloudflare,
    ip: &Ipv6Addr,
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

    for subdomain in &cloudflare.config.subdomains {
        // Construct the full domain name for logging
        let full_domain = if subdomain.name.is_empty() {
            cloudflare.config.name.clone()
        } else {
            format!("{}.{}", subdomain.name, cloudflare.config.name)
        };

        info!(
            zone = %cloudflare.config.name,
            domain = %full_domain,
            "Processing IPv6 DNS records"
        );

        'retry: loop {
            match process_domain_record_v6(cloudflare, &full_domain, ip).await {
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
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
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
            "Successfully processed {} IPv6 DNS records",
            update_count
        );
    }

    if let Some(error) = last_error {
        Err(error)
    } else {
        Ok(())
    }
}

/// Process a single domain record for IPv6 - fetch, create if missing, or update if needed
async fn process_domain_record_v6(
    cloudflare: &Cloudflare,
    full_domain: &str,
    ip: &Ipv6Addr,
) -> Result<(), CloudflareError> {
    let records = cloudflare
        .with_rate_limit(fetch_dns_records_v6(cloudflare, full_domain))
        .await?;

    if records.result.is_empty() {
        warn!(
            zone = %cloudflare.config.name,
            domain = %full_domain,
            "No AAAA records found, attempting to create"
        );
        return cloudflare
            .with_rate_limit(create_dns_record_v6(cloudflare, full_domain, ip))
            .await;
    }

    for record in records.result {
        if record.content != ip.to_string() {
            info!(
                zone = %cloudflare.config.name,
                domain = %full_domain,
                "Updating IPv6 DNS record from {} to {}",
                record.content,
                ip
            );

            match cloudflare
                .with_rate_limit(update_record_v6(cloudflare, &record.id, ip))
                .await
            {
                Ok(_) => {
                    info!(
                        zone = %cloudflare.config.name,
                        domain = %full_domain,
                        "Successfully updated IPv6 DNS record to {}",
                        ip
                    );
                }
                Err(e) => {
                    error!(
                        zone = %cloudflare.config.name,
                        domain = %full_domain,
                        "Failed to update IPv6 DNS record: {}",
                        e
                    );
                    return Err(e);
                }
            }
        } else {
            debug!(
                zone = %cloudflare.config.name,
                domain = %full_domain,
                "IPv6 DNS record already set to {}",
                ip
            );
        }
    }

    Ok(())
}

/// Fetches AAAA DNS records for a specific domain.
async fn fetch_dns_records_v6(
    cloudflare: &Cloudflare,
    domain: &str,
) -> Result<DnsResponse, CloudflareError> {
    let url = format!(
        "{}/zones/{}/dns_records?type=AAAA&name={}",
        CLOUDFLARE_API_BASE, cloudflare.config.zone_id, domain
    );

    debug!(
        zone = %cloudflare.config.name,
        domain = %domain,
        url = %url,
        "Sending IPv6 DNS records request"
    );

    let response = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        cloudflare.client.get(&url).send(),
    )
    .await
    .map_err(|_| CloudflareError::Timeout {
        zone: cloudflare.config.name.clone(),
        message: "IPv6 DNS record fetch request timed out".to_string(),
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
                "Received IPv6 DNS records response"
            );

            serde_json::from_str(&response_text).map_err(|e| CloudflareError::FetchFailed {
                zone: cloudflare.config.name.clone(),
                message: format!("Failed to parse response: {} - Raw: {}", e, response_text),
            })
        }
        StatusCode::UNAUTHORIZED => Err(CloudflareError::InvalidApiToken(
            cloudflare.config.name.clone(),
        )),
        StatusCode::NOT_FOUND => Err(CloudflareError::FetchFailed {
            zone: cloudflare.config.name.clone(),
            message: format!("Zone or DNS record not found for domain {}", domain),
        }),
        StatusCode::TOO_MANY_REQUESTS => {
            Err(CloudflareError::RateLimited(cloudflare.config.name.clone()))
        }
        _ => {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(CloudflareError::FetchFailed {
                zone: cloudflare.config.name.clone(),
                message: format!("HTTP {} - {}", status, error_body),
            })
        }
    }
}

/// Creates a new IPv6 DNS record
async fn create_dns_record_v6(
    cloudflare: &Cloudflare,
    domain: &str,
    ip: &Ipv6Addr,
) -> Result<(), CloudflareError> {
    info!(
        zone = %cloudflare.config.name,
        domain = %domain,
        "Creating new IPv6 DNS record with IP {}",
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
            "type": "AAAA",
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
        "Successfully created IPv6 DNS record"
    );
    Ok(())
}

/// Updates a specific IPv6 DNS record
async fn update_record_v6(
    cloudflare: &Cloudflare,
    record_id: &str,
    ip: &Ipv6Addr,
) -> Result<(), CloudflareError> {
    let url = format!(
        "{}/zones/{}/dns_records/{}",
        CLOUDFLARE_API_BASE, cloudflare.config.zone_id, record_id
    );

    let response = cloudflare
        .client
        .patch(&url)
        .json(&json!({
            "type": "AAAA",
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
