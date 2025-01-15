// Standard library
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

// 3rd party crates
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

// Project imports
use crate::utility::rate_limiter::traits::RateLimiter;
use crate::utility::rate_limiter::types::{RateLimitConfig, TokenBucketRateLimiter};

// Current module imports
use super::constants::{
    DEFAULT_MAX_NETWORK_RETRY_INTERVAL, DEFAULT_MAX_REQUESTS_PER_HOUR, DEFAULT_MIN_CONSENSUS,
    IPV4_SERVICES, IPV6_SERVICES, MAX_CONSECUTIVE_FAILURES, MAX_RETRIES, REQUEST_TIMEOUT_SECS,
    RETRY_DELAY_MS, SUSPENSION_DURATION_SECS,
};
use super::errors::{IpDetectionError, IpDetectionValidationError};
use super::traits::IpVersionOps;
use super::types::{
    IpDetection, IpDetector, IpResponse, IpService, IpVersion, VersionSuspension, V4, V6,
};

impl Default for IpDetection {
    fn default() -> Self {
        Self {
            max_requests_per_hour: DEFAULT_MAX_REQUESTS_PER_HOUR,
            min_consensus: DEFAULT_MIN_CONSENSUS,
            network_retry_interval: DEFAULT_MAX_NETWORK_RETRY_INTERVAL,
        }
    }
}

impl IpDetection {
    pub fn validate(&self) -> Result<(), IpDetectionValidationError> {
        // Validate max_requests_per_hour (must be > 0)
        if self.max_requests_per_hour == 0 {
            return Err(IpDetectionValidationError::InvalidMaxRequests(
                "must be greater than 0".into(),
            ));
        }

        // Validate min_consensus (must be > 0 and <= total number of services)
        if self.min_consensus == 0 {
            return Err(IpDetectionValidationError::InvalidMinConsensus(
                "must be greater than 0".into(),
            ));
        }

        // Get total number of services (IPv4 + IPv6)
        let total_services = IPV4_SERVICES.len() + IPV6_SERVICES.len();
        if self.min_consensus as usize > total_services {
            return Err(IpDetectionValidationError::InvalidMinConsensus(format!(
                "cannot be greater than total number of services ({})",
                total_services
            )));
        }

        // Validate network_retry_interval (must be > 0 and <= max allowed)
        if self.network_retry_interval == 0 {
            return Err(IpDetectionValidationError::InvalidRetryInterval(
                "must be greater than 0".into(),
            ));
        }
        if self.network_retry_interval > DEFAULT_MAX_NETWORK_RETRY_INTERVAL {
            return Err(IpDetectionValidationError::InvalidRetryInterval(format!(
                "cannot be greater than maximum allowed ({})",
                DEFAULT_MAX_NETWORK_RETRY_INTERVAL
            )));
        }

        Ok(())
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
            suspended_versions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Detects the current public IP address with consensus validation
    pub async fn detect_ip(&self, ip_version: IpVersion) -> Result<IpAddr, IpDetectionError> {
        // Check if version is suspended
        if let Some(suspension) = self.suspended_versions.read().await.get(&ip_version) {
            let elapsed = suspension.suspended_since.elapsed();
            if elapsed.as_secs() < SUSPENSION_DURATION_SECS {
                debug!(
                    "{:?} detection suspended for {} more seconds",
                    ip_version,
                    SUSPENSION_DURATION_SECS - elapsed.as_secs()
                );
                return Err(IpDetectionError::VersionSuspended {
                    version: ip_version,
                    remaining_secs: SUSPENSION_DURATION_SECS - elapsed.as_secs(),
                });
            }
            // Suspension duration expired, remove suspension
            self.suspended_versions.write().await.remove(&ip_version);
        }

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
        let version = V::version();

        // Helper function to check consensus and cleanup
        let check_consensus_and_cleanup =
            |responses: &[IpResponse],
             version: IpVersion,
             rate_limiter_idx: usize,
             suspended_versions: &Arc<RwLock<HashMap<IpVersion, VersionSuspension>>>|
             -> Option<Result<IpAddr, IpDetectionError>> {
                if let Ok(consensus_ip) = self.check_consensus(responses, min_consensus) {
                    // Clone the Arc before moving into the spawned task
                    let suspended_versions = Arc::clone(suspended_versions);
                    let rate_limiter = Arc::clone(&self.rate_limiters[rate_limiter_idx]);
                    tokio::spawn(async move {
                        rate_limiter.release().await;
                        suspended_versions.write().await.remove(&version);
                    });
                    return Some(Ok(consensus_ip));
                }
                None
            };

        // Helper function to query a service and handle responses
        async fn query_service<'a>(
            detector: &'a IpDetector,
            service: &'a IpService,
            rate_limiter_idx: usize,
            version: IpVersion,
            responses: &mut Vec<IpResponse>,
            errors: &mut Vec<IpDetectionError>,
            check_consensus: impl Fn(&[IpResponse]) -> Option<Result<IpAddr, IpDetectionError>>,
        ) -> Option<Result<IpAddr, IpDetectionError>> {
            // Check rate limit
            if !detector.rate_limiters[rate_limiter_idx].acquire().await {
                errors.push(IpDetectionError::RateLimitExceeded {
                    service: service.base_url.to_string(),
                });
                return None;
            }

            let result = match detector.query_ip_service_with_retry(service, version).await {
                Ok(ip) => {
                    debug!(
                        "Successfully got IP {} from service {}",
                        ip, service.base_url
                    );
                    responses.push(IpResponse {
                        ip,
                        is_primary: service.is_primary,
                    });

                    // Check if we have consensus
                    check_consensus(responses)
                }
                Err(e) => {
                    error!("Failed to query IP service {}: {}", service.base_url, e);
                    errors.push(e);
                    None
                }
            };

            detector.rate_limiters[rate_limiter_idx].release().await;
            result
        }

        // Helper function to try services until consensus is reached
        async fn try_services<'a>(
            detector: &'a IpDetector,
            services: &[&'a IpService],
            base_offset: usize,
            version: IpVersion,
            responses: &mut Vec<IpResponse>,
            errors: &mut Vec<IpDetectionError>,
            suspended_versions: &Arc<RwLock<HashMap<IpVersion, VersionSuspension>>>,
            check_consensus_and_cleanup: impl Fn(
                &[IpResponse],
                IpVersion,
                usize,
                &Arc<RwLock<HashMap<IpVersion, VersionSuspension>>>,
            )
                -> Option<Result<IpAddr, IpDetectionError>>,
        ) -> Option<Result<IpAddr, IpDetectionError>> {
            for (idx, service) in services.iter().enumerate() {
                let rate_limiter_idx = idx + base_offset;
                if let Some(result) = query_service(
                    detector,
                    service,
                    rate_limiter_idx,
                    version,
                    responses,
                    errors,
                    |responses| {
                        check_consensus_and_cleanup(
                            responses,
                            version,
                            rate_limiter_idx,
                            suspended_versions,
                        )
                    },
                )
                .await
                {
                    return Some(result);
                }
            }
            None
        }

        // Try primary services first
        let primary_services: Vec<_> = services.iter().filter(|s| s.is_primary).collect();
        if let Some(result) = try_services(
            self,
            &primary_services,
            offset,
            version,
            &mut responses,
            &mut errors,
            &self.suspended_versions,
            check_consensus_and_cleanup,
        )
        .await
        {
            return result;
        }

        // If no consensus from primary services, try secondary services
        let secondary_services: Vec<_> = services.iter().filter(|s| !s.is_primary).collect();
        if let Some(result) = try_services(
            self,
            &secondary_services,
            offset + primary_services.len(),
            version,
            &mut responses,
            &mut errors,
            &self.suspended_versions,
            check_consensus_and_cleanup,
        )
        .await
        {
            return result;
        }

        // Handle failures and suspension
        let mut suspended_versions = self.suspended_versions.write().await;
        match suspended_versions.get_mut(&version) {
            Some(suspension) => {
                suspension.consecutive_failures += 1;
                if suspension.consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                    suspension.suspended_since = Instant::now();
                    warn!(
                        "{:?} detection suspended for {} seconds after {} consecutive failures",
                        version, SUSPENSION_DURATION_SECS, suspension.consecutive_failures
                    );
                }
            }
            None => {
                suspended_versions.insert(version, VersionSuspension::new());
                warn!("First failure for {:?} detection", version);
            }
        }
        drop(suspended_versions);

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

impl VersionSuspension {
    pub fn new() -> Self {
        Self {
            suspended_since: Instant::now(),
            consecutive_failures: 1,
        }
    }
}
