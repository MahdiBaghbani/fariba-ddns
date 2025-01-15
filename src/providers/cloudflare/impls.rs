// Project modules
use crate::settings::errors::ValidationError;

use super::types::CfConfig;

impl CfConfig {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.zone_id.trim().is_empty() {
            return Err(ValidationError::CloudflareConfig(
                "zone_id cannot be empty".into(),
            ));
        }

        if self.api_token.trim().is_empty() {
            return Err(ValidationError::CloudflareConfig(
                "api_token cannot be empty".into(),
            ));
        }

        if self.name.trim().is_empty() {
            return Err(ValidationError::CloudflareConfig(
                "name cannot be empty".into(),
            ));
        }

        if self.subdomains.is_empty() {
            return Err(ValidationError::CloudflareConfig(
                "at least one subdomain must be configured".into(),
            ));
        }

        if self.rate_limit.max_requests == 0 {
            return Err(ValidationError::CloudflareConfig(
                "rate limit max_requests must be greater than 0".into(),
            ));
        }

        if self.rate_limit.window_secs == 0 {
            return Err(ValidationError::CloudflareConfig(
                "rate limit window_secs must be greater than 0".into(),
            ));
        }

        Ok(())
    }
}
