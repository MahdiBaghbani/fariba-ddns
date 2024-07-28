use serde::{Deserialize, Serialize};

pub mod providers;
pub mod resolvers;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub frequency: u64,
    pub dns_providers: providers::DnsProviders,
    pub dns_resolvers: resolvers::DnsResolvers,
}
