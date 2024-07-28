use serde::{Deserialize, Serialize};

pub mod ident_me;

#[derive(Serialize, Deserialize)]
pub struct DnsResolvers {
    #[serde(rename = "ident.me")]
    pub ident_me: Option<ident_me::IdentMe>,
}
