use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tari_common_types::tari_address::TariAddress;

use crate::db_types::{MicroTari, Order};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResult {
    #[serde(serialize_with = "address_to_hex")]
    pub address: TariAddress,
    pub total_orders: MicroTari,
    pub orders: Vec<Order>,
}

pub fn address_to_hex<S>(address: &TariAddress, serializer: S) -> Result<S::Ok, S::Error>
where S: serde::Serializer {
    serializer.serialize_str(&address.to_hex())
}

pub fn str_to_address<'de, D>(deserializer: D) -> Result<TariAddress, D::Error>
where D: serde::Deserializer<'de> {
    let s = String::deserialize(deserializer)?;
    TariAddress::from_str(&s).map_err(serde::de::Error::custom)
}
