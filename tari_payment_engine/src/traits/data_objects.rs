use std::{fmt::Display, net::IpAddr};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tpg_common::MicroTari;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAccountPayment {
    pub address: SerializedTariAddress,
    pub orders_paid: Vec<Order>,
    /// An array of account ids used to pay for the orders, as well as the amount paid from each account
    pub wallet_accounts_used: Vec<(i64, MicroTari)>,
}

impl MultiAccountPayment {
    pub fn new(address: SerializedTariAddress, orders_paid: Vec<Order>, accounts: &[(i64, MicroTari)]) -> Self {
        Self { address, orders_paid, wallet_accounts_used: accounts.to_vec() }
    }

    pub fn order_count(&self) -> usize {
        self.orders_paid.len()
    }

    pub fn account_count(&self) -> usize {
        self.wallet_accounts_used.len()
    }

    pub fn total_paid(&self) -> MicroTari {
        self.wallet_accounts_used.iter().map(|(_, amount)| *amount).sum()
    }
}

impl Display for MultiAccountPayment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MultiAccountPayment from address {}:", self.address.as_address())?;
        writeln!(
            f,
            "{} orders paid from {} accounts for a total of {}",
            self.order_count(),
            self.account_count(),
            self.total_paid()
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpiryResult {
    pub unclaimed: Vec<Order>,
    pub unpaid: Vec<Order>,
}

impl ExpiryResult {
    pub fn new(unclaimed: Vec<Order>, unpaid: Vec<Order>) -> Self {
        Self { unclaimed, unpaid }
    }

    pub fn unclaimed_count(&self) -> usize {
        self.unclaimed.len()
    }

    pub fn unpaid_count(&self) -> usize {
        self.unpaid.len()
    }

    pub fn total_count(&self) -> usize {
        self.unclaimed_count() + self.unpaid_count()
    }
}
