use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::db_types::SerializedTariAddress;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWalletInfo {
    pub address: SerializedTariAddress,
    pub ip_address: IpAddr,
    pub initial_nonce: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WalletInfo {
    pub address: SerializedTariAddress,
    pub ip_address: IpAddr,
    pub last_nonce: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWalletInfo {
    pub address: Option<SerializedTariAddress>,
    pub ip_address: Option<IpAddr>,
}
