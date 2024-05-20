use serde::{Deserialize, Serialize};
use tari_payment_engine::db_types::Role;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleUpdateRequest {
    pub address: String,
    #[serde(default)]
    pub apply: Vec<Role>,
    #[serde(default)]
    pub revoke: Vec<Role>,
}
