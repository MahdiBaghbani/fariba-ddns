// Standard library
use std::error::Error;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::RwLockReadGuard;
use tracing::{debug, error};

// External crates
use futures::{stream::FuturesUnordered, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{header, Client};

// Project modules
use crate::settings::structs::{ConfigManager, Settings};

use super::errors::CloudflareError;
use super::structs::{CfConfig, Cloudflare};

/// Creates a reqwest client with the appropriate headers for Cloudflare API.
///
/// # Errors
///
/// Returns an error if the API token is invalid or if the client cannot be built.
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

/// Processes updates concurrently.
///
/// # Parameters
///
/// - `cloudflares`: A slice of `Cloudflare` instances to update.
/// - `ip`: The new public IPv4 address to update the DNS records with.
///
/// # Errors
///
/// Returns an error if any of the updates fail.
///
/// # Examples
///
/// ```rust
/// process_cloudflare_updates(&cloudflares, ip).await?;
/// ```
pub async fn process_updates(
    cloudflares: &[Cloudflare],
    ip: &Ipv4Addr,
) -> Result<(), Box<dyn Error>> {
    // Create a FuturesUnordered to hold our concurrent tasks.
    let mut futures = FuturesUnordered::new();

    // For each Cloudflare instance, spawn an async task to update DNS records.
    for cloudflare in cloudflares {
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
            }
        }
    }

    Ok(())
}

pub async fn get_cloudflares(config: Arc<ConfigManager>) -> Result<Vec<Cloudflare>, CloudflareError> {
    let settings: RwLockReadGuard<Settings> = config.get_settings().await;

    // Convert the vector of `CloudflareConfig` into a vector of `Cloudflare`.
    let configs_to_cloudflares: Result<Vec<Option<Cloudflare>>, CloudflareError> = settings
        .get_cloudflares()
        .into_iter()
        .map(Cloudflare::new)
        .collect();

    configs_to_cloudflares.map(|vec| vec.into_iter().flatten().collect())
}
