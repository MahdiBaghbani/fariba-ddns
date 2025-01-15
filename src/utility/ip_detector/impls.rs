// Standard library
use std::collections::HashMap;
use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

// 3rd party crates
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

// Project imports
use crate::utility::rate_limiter::traits::RateLimiter;
use crate::utility::rate_limiter::types::{RateLimitConfig, TokenBucketRateLimiter};

use super::constants::{
    DEFAULT_MAX_NETWORK_RETRY_INTERVAL, DEFAULT_MAX_REQUESTS_PER_HOUR, DEFAULT_MIN_CONSENSUS,
    IPV4_SERVICES, IPV6_SERVICES, MAX_RETRIES, REQUEST_TIMEOUT_SECS, RETRY_DELAY_MS,
};
use super::errors::IpDetectionError;
use super::traits::IpVersionOps;
use super::types::{IpDetection, IpDetector, IpResponse, IpService, IpVersion, V4, V6};

impl Default for IpDetection {
    fn default() -> Self {
        Self {
            max_requests_per_hour: DEFAULT_MAX_REQUESTS_PER_HOUR,
            min_consensus: DEFAULT_MIN_CONSENSUS,
            network_retry_interval: DEFAULT_MAX_NETWORK_RETRY_INTERVAL,
        }
    }
}

impl IpDetector {
    pub fn new(config: IpDetection) -> Self {
        // Create rate limiters for both IPv4 and IPv6 services
        let mut rate_limiters = Vec::new();
        rate_limiters.extend(V4::get_services().iter().map(|_| {
            Arc::new(TokenBucketRateLimiter::new(RateLimitConfig {
                max_requests: config.max_requests_per_hour,
                window_secs: 3600, // 1 hour
            })) as Arc<dyn RateLimiter>
        }));
        rate_limiters.extend(V6::get_services().iter().map(|_| {
            Arc::new(TokenBucketRateLimiter::new(RateLimitConfig {
                max_requests: config.max_requests_per_hour,
                window_secs: 3600, // 1 hour
            })) as Arc<dyn RateLimiter>
        }));

        Self {
            config,
            rate_limiters,
            last_check: Arc::new(RwLock::new(Instant::now())),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
                .user_agent("fariba-ddns/1.0")
                .build()
                .unwrap_or_default(),
        }
    }

    /// Detects the current public IP address with consensus validation
    pub async fn detect_ip(&self, ip_version: IpVersion) -> Result<IpAddr, IpDetectionError> {
        match ip_version {
            IpVersion::V4 => self.detect_ip_for_version::<V4>().await,
            IpVersion::V6 => self.detect_ip_for_version::<V6>().await,
        }
    }

    /// Generic IP detection for a specific version
    async fn detect_ip_for_version<V: IpVersionOps>(&self) -> Result<IpAddr, IpDetectionError> {
        let mut responses = Vec::new();
        let mut errors = Vec::new();
        let services = V::get_services();
        let offset = V::rate_limiter_offset();
        let min_consensus = self.config.min_consensus as usize;

        // Try primary services first
        let primary_services: Vec<_> = services.iter().filter(|s| s.is_primary).collect();
        for (idx, service) in primary_services.iter().enumerate() {
            let rate_limiter_idx = idx + offset;

            // Check rate limit
            if !self.rate_limiters[rate_limiter_idx].acquire().await {
                errors.push(IpDetectionError::RateLimitExceeded {
                    service: service.base_url.to_string(),
                });
                continue;
            }

            match self
                .query_ip_service_with_retry(service, V::version())
                .await
            {
                Ok(ip) => {
                    debug!(
                        "Successfully got IP {} from service {}",
                        ip, service.base_url
                    );
                    responses.push(IpResponse {
                        ip,
                        is_primary: true,
                    });

                    // Check if we have consensus from primary services
                    if let Ok(consensus_ip) = self.check_consensus(&responses, min_consensus) {
                        self.rate_limiters[rate_limiter_idx].release().await;
                        return Ok(consensus_ip);
                    }
                }
                Err(e) => {
                    error!("Failed to query IP service {}: {}", service.base_url, e);
                    errors.push(e);
                }
            }

            self.rate_limiters[rate_limiter_idx].release().await;
        }

        // If no consensus from primary services, try secondary services
        let secondary_services: Vec<_> = services.iter().filter(|s| !s.is_primary).collect();
        for (idx, service) in secondary_services.iter().enumerate() {
            let rate_limiter_idx = idx + offset + primary_services.len();

            // Check rate limit
            if !self.rate_limiters[rate_limiter_idx].acquire().await {
                errors.push(IpDetectionError::RateLimitExceeded {
                    service: service.base_url.to_string(),
                });
                continue;
            }

            match self
                .query_ip_service_with_retry(service, V::version())
                .await
            {
                Ok(ip) => {
                    debug!(
                        "Successfully got IP {} from service {}",
                        ip, service.base_url
                    );
                    responses.push(IpResponse {
                        ip,
                        is_primary: false,
                    });

                    // Check if we have consensus with all responses
                    if let Ok(consensus_ip) = self.check_consensus(&responses, min_consensus) {
                        self.rate_limiters[rate_limiter_idx].release().await;
                        return Ok(consensus_ip);
                    }
                }
                Err(e) => {
                    error!("Failed to query IP service {}: {}", service.base_url, e);
                    errors.push(e);
                }
            }

            self.rate_limiters[rate_limiter_idx].release().await;
        }

        if responses.is_empty() {
            return Err(IpDetectionError::NoServicesAvailable);
        }

        // If we get here, we don't have consensus
        Err(IpDetectionError::ConsensusNotReached {
            responses: responses.len(),
            required: self.config.min_consensus,
        })
    }

    /// Check if we have consensus among the responses
    fn check_consensus(
        &self,
        responses: &[IpResponse],
        min_consensus: usize,
    ) -> Result<IpAddr, IpDetectionError> {
        let mut ip_counts = HashMap::new();
        for response in responses {
            *ip_counts.entry(response.ip).or_insert(0) += 1;

            // If any IP has reached the minimum consensus, return it
            if ip_counts[&response.ip] >= min_consensus {
                return Ok(response.ip);
            }
        }
        Err(IpDetectionError::ConsensusNotReached {
            responses: responses.len(),
            required: self.config.min_consensus,
        })
    }

    /// Checks network connectivity with retries
    pub async fn check_network(&self) -> bool {
        // Try IPv4 services first
        if self.check_network_for_version::<V4>().await {
            return true;
        }
        // Fall back to IPv6 services
        self.check_network_for_version::<V6>().await
    }

    /// Generic network check for a specific version
    async fn check_network_for_version<V: IpVersionOps>(&self) -> bool {
        let services = V::get_services();
        let offset = V::rate_limiter_offset();

        for (idx, service) in services.iter().enumerate() {
            let rate_limiter_idx = idx + offset;
            if !self.rate_limiters[rate_limiter_idx].acquire().await {
                continue;
            }

            for retry in 0..MAX_RETRIES {
                match self.client.get(service.base_url).send().await {
                    Ok(_) => {
                        self.rate_limiters[rate_limiter_idx].release().await;
                        return true;
                    }
                    Err(e) => {
                        if retry < MAX_RETRIES - 1 {
                            warn!(
                                "Network check failed for {}, retrying: {}",
                                service.base_url, e
                            );
                            tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                            continue;
                        }
                        error!(
                            "Network check failed for {} after {} retries: {}",
                            service.base_url, MAX_RETRIES, e
                        );
                    }
                }
            }
            self.rate_limiters[rate_limiter_idx].release().await;
        }
        false
    }

    /// Gets the network retry interval from the configuration
    pub fn get_network_retry_interval(&self) -> u64 {
        self.config.network_retry_interval
    }

    /// Query IP service with retry logic
    async fn query_ip_service_with_retry(
        &self,
        service: &IpService,
        ip_version: IpVersion,
    ) -> Result<IpAddr, IpDetectionError> {
        let mut last_error = None;

        for retry in 0..MAX_RETRIES {
            return match self.query_ip_service(service, ip_version).await {
                Ok(ip) => Ok(ip),
                Err(e) => {
                    if retry < MAX_RETRIES - 1 {
                        warn!("Query failed for {}, retrying: {}", service.base_url, e);
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                        last_error = Some(e);
                        continue;
                    }
                    Err(e)
                }
            };
        }

        Err(last_error.unwrap_or(IpDetectionError::NoServicesAvailable))
    }

    async fn query_ip_service(
        &self,
        service: &IpService,
        ip_version: IpVersion,
    ) -> Result<IpAddr, IpDetectionError> {
        let url = format!("{}{}", service.base_url, service.path);
        let response =
            self.client
                .get(&url)
                .send()
                .await
                .map_err(|e| IpDetectionError::NetworkError {
                    service: service.base_url.to_string(),
                    error: e,
                })?;

        let text = response
            .text()
            .await
            .map_err(|e| IpDetectionError::NetworkError {
                service: service.base_url.to_string(),
                error: e,
            })?;

        // Try to parse as JSON first (for services that return JSON)
        if text.trim().starts_with('{') {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                // Try common JSON fields for IP addresses
                for field in ["ip", "address", "ipAddress", "query"] {
                    if let Some(ip_str) = json.get(field).and_then(|v| v.as_str()) {
                        if let Ok(ip) = ip_str.parse() {
                            return self.validate_ip_version(ip, ip_version, service);
                        }
                    }
                }
            }
        }

        // Try direct parsing if not JSON or JSON parsing failed
        text.trim()
            .parse()
            .map_err(|e: std::net::AddrParseError| IpDetectionError::ParseError {
                service: service.base_url.to_string(),
                error: e.to_string(),
            })
            .and_then(|ip| self.validate_ip_version(ip, ip_version, service))
    }

    fn validate_ip_version(
        &self,
        ip: IpAddr,
        expected_version: IpVersion,
        service: &IpService,
    ) -> Result<IpAddr, IpDetectionError> {
        match (ip, expected_version) {
            (IpAddr::V4(_), IpVersion::V4) | (IpAddr::V6(_), IpVersion::V6) => Ok(ip),
            (got_ip, _) => Err(IpDetectionError::VersionMismatch {
                service: service.base_url.to_string(),
                expected: expected_version,
                got: if matches!(got_ip, IpAddr::V4(_)) {
                    IpVersion::V4
                } else {
                    IpVersion::V6
                },
            }),
        }
    }
}

impl IpVersionOps for V4 {
    fn get_services() -> &'static [IpService] {
        &IPV4_SERVICES
    }
    fn rate_limiter_offset() -> usize {
        0
    }
    fn version() -> IpVersion {
        IpVersion::V4
    }
}

impl IpVersionOps for V6 {
    fn get_services() -> &'static [IpService] {
        &IPV6_SERVICES
    }
    fn rate_limiter_offset() -> usize {
        IPV4_SERVICES.len()
    }
    fn version() -> IpVersion {
        IpVersion::V6
    }
}

impl std::error::Error for IpDetectionError {}

impl fmt::Display for IpDetectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkError { service, error } => {
                write!(f, "Network error from {}: {}", service, error)
            }
            Self::InvalidResponse { service, response } => {
                write!(f, "Invalid response from {}: {}", service, response)
            }
            Self::VersionMismatch {
                service,
                expected,
                got,
            } => write!(
                f,
                "IP version mismatch from {}: expected {:?}, got {:?}",
                service, expected, got
            ),
            Self::RateLimitExceeded { service } => {
                write!(f, "Rate limit exceeded for {}", service)
            }
            Self::ParseError { service, error } => {
                write!(f, "Parse error from {}: {}", service, error)
            }
            Self::ConsensusNotReached {
                responses,
                required,
            } => write!(
                f,
                "Consensus not reached: got {} responses, need {}",
                responses, required
            ),
            Self::NoServicesAvailable => write!(f, "No IP detection services available"),
        }
    }
}
