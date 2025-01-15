// Current module imports
use super::errors::CloudflareValidationError;
use super::types::CfConfig;

impl CfConfig {
    pub fn validate(&self) -> Result<(), CloudflareValidationError> {
        if self.zone_id.trim().is_empty() {
            return Err(CloudflareValidationError::MissingZoneId);
        }

        if self.api_token.trim().is_empty() {
            return Err(CloudflareValidationError::MissingApiToken);
        }

        if self.name.trim().is_empty() {
            return Err(CloudflareValidationError::MissingName);
        }

        if self.subdomains.is_empty() {
            return Err(CloudflareValidationError::NoSubdomains);
        }

        if self.rate_limit.max_requests == 0 {
            return Err(CloudflareValidationError::InvalidRateLimit(
                "max_requests must be greater than 0".into(),
            ));
        }

        if self.rate_limit.window_secs == 0 {
            return Err(CloudflareValidationError::InvalidRateLimit(
                "window_secs must be greater than 0".into(),
            ));
        }

        Ok(())
    }
}
