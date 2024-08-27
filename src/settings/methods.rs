use std::env;

use config::{Config, ConfigError, Environment, File};

use crate::settings::models::{Cloudflare, CloudflareSubDomain, Settings};

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let file_path: String = env::var("FDDNS_CONFIG_PATH").unwrap();

        let settings: Config = Config::builder()
            .add_source(File::with_name(&file_path))
            // add in settings from the environment (with a prefix of APP)
            // eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
            .add_source(Environment::with_prefix("FDDNS"))
            .build()?;

        // deserialize and thus freeze the entire configuration.
        settings.try_deserialize()
    }
}

impl Clone for Cloudflare {
    fn clone(&self) -> Self {
        Cloudflare {
            enabled: self.enabled,
            name: self.name.clone(),
            zone_id: self.zone_id.clone(),
            api_token: self.api_token.clone(),
            subdomains: self.subdomains.clone(),
        }
    }
}

impl Clone for CloudflareSubDomain {
    fn clone(&self) -> Self {
        CloudflareSubDomain {
            name: self.name.clone(),
        }
    }
}
