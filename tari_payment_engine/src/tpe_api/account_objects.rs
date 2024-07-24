use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::db_types::{Order, Payment, SerializedTariAddress, UserAccount};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FullAccount {
    pub account: UserAccount,
    pub addresses: Vec<AccountAddress>,
    pub customer_ids: Vec<CustomerId>,
    pub orders: Vec<Order>,
    pub payments: Vec<Payment>,
}

impl FullAccount {
    pub fn new(account: UserAccount) -> Self {
        Self { account, ..Default::default() }
    }

    pub fn with_addresses(mut self, addresses: Vec<AccountAddress>) -> Self {
        self.addresses = addresses;
        self
    }

    pub fn with_customer_ids(mut self, customer_ids: Vec<CustomerId>) -> Self {
        self.customer_ids = customer_ids;
        self
    }

    pub fn with_orders(mut self, orders: Vec<Order>) -> Self {
        self.orders = orders;
        self
    }

    pub fn with_payments(mut self, payments: Vec<Payment>) -> Self {
        self.payments = payments;
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
pub struct AccountAddress {
    pub address: SerializedTariAddress,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
pub struct CustomerId {
    pub customer_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub offset: Option<i64>,
    pub count: Option<i64>,
}
