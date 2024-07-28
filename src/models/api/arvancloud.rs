use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ArvanDNSData {
    pub data: Vec<ArvanDNSRecord>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArvanDNSRecord {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub name: String,
    pub value: Vec<ArvanIPv4Record>,
    pub ttl: i64,
    pub cloud: bool,
    pub upstream_https: String,
    pub ip_filter_mode: ArvanIPFilterMode,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArvanIPv4Record {
    pub ip: String,
    pub port: Option<i64>,
    pub weight: i64,
    pub original_weight: Option<i64>,
    pub country: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArvanIPFilterMode {
    pub count: String,
    pub order: String,
    pub geo_filter: String,
}
