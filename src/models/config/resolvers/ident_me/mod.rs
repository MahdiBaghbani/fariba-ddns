use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct IdentMe {
    pub enabled: bool,
}
