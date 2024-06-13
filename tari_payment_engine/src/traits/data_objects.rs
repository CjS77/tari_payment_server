use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{
    db_types::{Order, SerializedTariAddress},
    order_objects::OrderChanged,
};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderMovedResult {
    pub orders: OrderChanged,
    pub old_account_id: i64,
    pub new_account_id: i64,
    pub is_filled: bool,
}

impl OrderMovedResult {
    pub fn new(old_account_id: i64, new_account_id: i64, old_order: Order, new_order: Order, is_filled: bool) -> Self {
        let orders = OrderChanged::new(old_order, new_order);
        Self { orders, old_account_id, new_account_id, is_filled }
    }

    pub fn filled_order(&self) -> Option<Order> {
        if self.is_filled {
            Some(self.orders.new_order.clone())
        } else {
            None
        }
    }
}
