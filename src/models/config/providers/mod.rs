use serde::{Deserialize, Serialize};

pub mod arvancloud;
pub mod cloudflare;

#[derive(Serialize, Deserialize)]
pub struct DnsProviders {
    pub arvancloud: Option<arvancloud::Arvancloud>,
    pub cloudflare: Option<cloudflare::Cloudflare>,
}
