// Standard library
use std::net::Ipv4Addr;

// 3rd party crates
use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use tracing::{debug, error, info, trace};

// Current module imports
use super::errors::CloudflareError;
use super::functions::create_reqwest_client;
use super::structs::{CfConfig, CfSubDomain, Cloudflare, DnsResponse, ZoneResponse};

impl CfSubDomain {
    /// Updates the DNS record for the subdomain with the new IP address.
    ///
    /// # Parameters
    ///
    /// - `ip`: The new IPv4 address to set in the DNS record.
    /// - `zone_id`: The Cloudflare zone ID.
    /// - `zone_name`: The Cloudflare zone name.
    /// - `client`: The HTTP client to use for the request.
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    pub async fn update_dns_record(
        &self,
        ip: &Ipv4Addr,
        zone_id: &str,
        zone_name: &str,
        client: &Client,
    ) -> Result<(), CloudflareError> {
        // Construct the URL to list DNS records for the subdomain.
        let url: String = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records?type=A&name={}.{}",
            zone_id, self.name, zone_name
        );

        // Send the request to get existing DNS records.
        let list_response: Response = client.get(&url).send().await.map_err(|e| {
            error!(
                domain = %zone_name,
                subdomain = %self.name,
                "Failed to send request to list DNS records: {}",
                e
            );
            CloudflareError::HttpRequest(e)
        })?;

        if !list_response.status().is_success() {
            let status: StatusCode = list_response.status();
            let text: String = list_response.text().await.unwrap_or_default();
            error!(
                domain = %zone_name,
                subdomain = %self.name,
                status = %status,
                "Failed to retrieve DNS record ID: {}",
                text.trim()
            );
            return Err(CloudflareError::HttpStatusError {
                zone: zone_id.to_string(),
                status,
                message: text,
            });
        }

        // Parse the response to get the record ID.
        let list_data: DnsResponse = list_response.json::<DnsResponse>().await.map_err(|e| {
            error!(
                domain = %zone_name,
                subdomain = %self.name,
                "Failed to parse DNS record list response: {}",
                e
            );
            CloudflareError::ParsingError(self.name.clone(), e)
        })?;

        // Check if the DNS record exists.
        if let Some(record) = list_data.result.first() {
            if record.content != ip.to_string() {
                // DNS record exists, update it using PUT.
                let update_url: String = format!(
                    "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
                    zone_id, record.id
                );

                let body: Value = serde_json::json!({
                    "type": "A",
                    "name": self.name,
                    "content": ip.to_string(),
                     // Automatic TTL
                    "ttl": 1,
                    "proxied": false
                });

                let response: Response =
                    client
                        .put(&update_url)
                        .json(&body)
                        .send()
                        .await
                        .map_err(|e| {
                            error!(
                                domain = %zone_name,
                                subdomain = %self.name,
                                "Failed to send update request: {}",
                                e
                            );
                            CloudflareError::HttpRequest(e)
                        })?;

                if !response.status().is_success() {
                    let status: StatusCode = response.status();
                    let text: String = response.text().await.unwrap_or_default();
                    error!(
                        domain = %zone_name,
                        subdomain = %self.name,
                        status = %status,
                        "Failed to update DNS record: {}",
                        text.trim()
                    );
                    return Err(CloudflareError::HttpStatusError {
                        zone: zone_id.to_string(),
                        status,
                        message: text,
                    });
                }

                info!(
                    domain = %zone_name,
                    subdomain = %self.name,
                    "Successfully updated DNS record to IP {}",
                    ip
                );
            } else {
                info!(
                    domain = %zone_name,
                    subdomain = %self.name,
                    "This DNS record is already pointing to IP {}, no updates needed",
                    ip
                );
            }
        } else {
            // DNS record does not exist, create it using POST.
            let create_url: String = format!(
                "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
                zone_id
            );

            let body: Value = serde_json::json!({
                "type": "A",
                "name": self.name,
                "content": ip.to_string(),
                "ttl": 1, // Automatic TTL
                "proxied": false
            });

            let response: Response =
                client
                    .post(&create_url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| {
                        error!(
                            domain = %zone_name,
                            subdomain = %self.name,
                            "Failed to send create request: {}",
                            e
                        );
                        CloudflareError::HttpRequest(e)
                    })?;

            if !response.status().is_success() {
                let status: StatusCode = response.status();
                let text: String = response.text().await.unwrap_or_default();
                error!(
                    domain = %zone_name,
                    subdomain = %self.name,
                    status = %status,
                    "Failed to create DNS record: {}",
                    text.trim()
                );
                return Err(CloudflareError::HttpStatusError {
                    zone: zone_id.to_string(),
                    status,
                    message: text,
                });
            }

            debug!(
                domain = %zone_name,
                subdomain = %self.name,
                "Successfully created DNS record with IP {}",
                ip
            );
        }

        Ok(())
    }
}

impl CfConfig {
    /// Validates the Cloudflare configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if any required configuration is missing or invalid.
    pub fn validate(&self) -> Result<(), CloudflareError> {
        if self.name.is_empty() {
            return Err(CloudflareError::ConfigError(
                "Configuration 'name' is missing".to_string(),
            ));
        }
        if self.zone_id.is_empty() {
            return Err(CloudflareError::ConfigError(
                "Zone ID is missing".to_string(),
            ));
        }
        if self.api_token.is_empty() || self.api_token == "your_api_token_here" {
            return Err(CloudflareError::InvalidApiToken(self.name.clone()));
        }

        // Additional validation as needed
        Ok(())
    }
}

impl Cloudflare {
    /// Creates a new `Cloudflare` instance from a `CloudflareConfig`.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid or the client cannot be created.
    pub fn new(config: CfConfig) -> Result<Option<Self>, CloudflareError> {
        if config.enabled {
            config.validate()?;
            let client: Client = create_reqwest_client(&config)?;
            Ok(Some(Cloudflare { config, client }))
        } else {
            Ok(None)
        }
    }

    /// Retrieves information about the Cloudflare zone.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, the response cannot be parsed,
    /// or the zone is not active.
    pub async fn get_zone_info(&self) -> Result<(), CloudflareError> {
        let url: String = format!(
            "https://api.cloudflare.com/client/v4/zones/{}",
            self.config.zone_id
        );

        // Send the request to get zone information
        let response: Response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status: StatusCode = response.status();
            let text: String = response.text().await?;
            let message: String = format!("Failed to get zone info: {}", text.trim());
            error!(
                zone = %self.config.name,
                status = %status,
                "HTTP error: {}",
                message
            );
            return Err(CloudflareError::HttpStatusError {
                zone: self.config.name.clone(),
                status,
                message,
            });
        }

        let zone_data: ZoneResponse = response.json().await.map_err(|e| {
            error!(
                zone = %self.config.name,
                "Failed to parse zone response: {}",
                e
            );
            CloudflareError::ParsingError(self.config.name.clone(), e)
        })?;

        trace!(zone = %self.config.name, "Zone data: {:#?}", zone_data);

        if !zone_data.success {
            error!(
                zone = %self.config.name,
                "API response indicates failure (success field is false)"
            );
            return Err(CloudflareError::ApiResponseFailure(
                self.config.name.clone(),
            ));
        }

        if zone_data.result.status != "active" {
            error!(
                zone = %self.config.name,
                status = %zone_data.result.status,
                "Zone is not active (status: {})",
                zone_data.result.status
            );
            return Err(CloudflareError::InactiveZone(
                self.config.name.clone(),
                zone_data.result.status.clone(),
            ));
        }

        debug!(zone = %self.config.name, "Successfully retrieved zone info");

        Ok(())
    }

    /// Updates the DNS records for the Cloudflare instance with the new IP address.
    ///
    /// # Parameters
    ///
    /// - `ip`: The new IPv4 address to set in the DNS records.
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    pub async fn update_dns_records(&self, ip: &Ipv4Addr) -> Result<(), CloudflareError> {
        let mut futures = FuturesUnordered::new();

        self.get_zone_info().await?;

        for subdomain in &self.config.subdomains {
            futures.push(async move {
                // Build the API endpoint URL for updating DNS records.
                subdomain
                    .update_dns_record(ip, &self.config.zone_id, &self.config.name, &self.client)
                    .await?;

                Ok::<(), CloudflareError>(())
            });
        }

        // Process all futures.
        let mut errors: Vec<CloudflareError> = Vec::new();

        while let Some(result) = futures.next().await {
            if let Err(e) = result {
                errors.push(e);
            }
        }

        if !errors.is_empty() {
            // Return the first error or aggregate them as needed.
            return Err(CloudflareError::ApiResponseFailure(
                self.config.name.clone(),
            ));
        }

        Ok(())
    }
}
