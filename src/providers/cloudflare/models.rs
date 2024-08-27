use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ZoneResponse {
    pub result: ZoneResponseResult,
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct ZoneResponseResult {
    pub name: String,
    pub status: String,
}
