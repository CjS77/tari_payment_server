use serde::{Deserialize, Serialize};
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::Role,
    order_objects::{address_to_hex, str_to_address},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SerializedTariAddress(
    #[serde(serialize_with = "address_to_hex", deserialize_with = "str_to_address")] TariAddress,
);

impl SerializedTariAddress {
    pub fn to_address(self) -> TariAddress {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleUpdateRequest {
    pub address: String,
    #[serde(default)]
    pub apply: Vec<Role>,
    #[serde(default)]
    pub revoke: Vec<Role>,
}
