use std::{fmt::Display, net::IpAddr};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tpg_common::MicroTari;

use crate::{
    db_types::{Order, SerializedTariAddress, SettlementJournalEntry},
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
pub struct OrderMovedResult {
    pub orders: OrderChanged,
    pub settlements: Vec<SettlementJournalEntry>,
}

impl OrderMovedResult {
    pub fn new(old_order: Order, new_order: Order, settlements: Vec<SettlementJournalEntry>) -> Self {
        let orders = OrderChanged::new(old_order, new_order);
        Self { orders, settlements }
    }

    pub fn total_paid(&self) -> MicroTari {
        self.settlements.iter().map(|s| s.amount).sum()
    }

    pub fn is_filled(&self) -> bool {
        self.total_paid() >= self.orders.new_order.total_price
    }

    pub fn filled_order(&self) -> Option<Order> {
        if self.is_filled() {
            Some(self.orders.new_order.clone())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAccountPayment {
    pub orders_paid: Vec<Order>,
    /// An array of account ids used to pay for the orders, as well as the amount paid from each account
    pub settlements: Vec<SettlementJournalEntry>,
}

impl MultiAccountPayment {
    pub fn new(orders_paid: Vec<Order>, settlements: Vec<SettlementJournalEntry>) -> Self {
        Self { orders_paid, settlements }
    }

    pub fn order_count(&self) -> usize {
        self.orders_paid.len()
    }

    /// Converts this payment into the first order.
    ///
    /// Only call this if you know that this payment only contains exactly one order.
    ///
    /// Panics if there are no orders in this payment.
    pub fn to_order(mut self) -> Order {
        self.orders_paid.remove(0)
    }

    pub fn total_paid(&self) -> MicroTari {
        self.settlements.iter().map(|s| s.amount).sum()
    }

    /// Merge the payments into a single payment, using the first payment's address as the representative address.
    /// You can retrieve the other addresses from the `settlements` field.
    pub fn merge<I: IntoIterator<Item = MultiAccountPayment>>(payments: I) -> Option<Self> {
        let mut iter = payments.into_iter();
        let mut merged = iter.next()?;
        for next in iter {
            merged.orders_paid.extend(next.orders_paid);
            merged.settlements.extend(next.settlements);
        }
        Some(merged)
    }
}

impl Display for MultiAccountPayment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Multi-account payment. ")?;
        let n = self.settlements.len();
        writeln!(
            f,
            "{} orders paid from for a total of {} from {n} address(es).",
            self.order_count(),
            self.total_paid(),
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
