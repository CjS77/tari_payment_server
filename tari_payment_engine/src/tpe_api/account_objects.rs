use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{
    db_types::{
        AddressBalance,
        CustomerBalance,
        CustomerOrderBalance,
        Order,
        Payment,
        SerializedTariAddress,
        SettlementJournalEntry,
    },
    traits::AccountApiError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressHistory {
    pub address: SerializedTariAddress,
    pub balance: AddressBalance,
    pub orders: Vec<Order>,
    pub payments: Vec<Payment>,
    pub settlements: Vec<SettlementJournalEntry>,
}

impl AddressHistory {
    pub fn new(
        address: SerializedTariAddress,
        balance: AddressBalance,
        orders: Vec<Order>,
        payments: Vec<Payment>,
        settlements: Vec<SettlementJournalEntry>,
    ) -> Self {
        Self { address, balance, orders, payments, settlements }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerHistory {
    pub customer_id: String,
    pub balance: CustomerBalance,
    pub order_balance: CustomerOrderBalance,
    pub orders: Vec<Order>,
    pub settlements: Vec<SettlementJournalEntry>,
}

#[derive(Default)]
pub struct CustomerHistoryBuilder {
    customer_id: String,
    balance: Option<CustomerBalance>,
    order_balance: Option<CustomerOrderBalance>,
    orders: Option<Vec<Order>>,
    settlements: Option<Vec<SettlementJournalEntry>>,
}

impl CustomerHistoryBuilder {
    pub fn balance(mut self, balance: CustomerBalance) -> Self {
        self.balance = Some(balance);
        self
    }

    pub fn order_balance(mut self, order_balance: CustomerOrderBalance) -> Self {
        self.order_balance = Some(order_balance);
        self
    }

    pub fn orders(mut self, orders: Vec<Order>) -> Self {
        self.orders = Some(orders);
        self
    }

    pub fn settlements(mut self, settlements: Vec<SettlementJournalEntry>) -> Self {
        self.settlements = Some(settlements);
        self
    }

    pub fn build(self) -> Result<CustomerHistory, AccountApiError> {
        let history = CustomerHistory {
            customer_id: self.customer_id,
            balance: self
                .balance
                .ok_or_else(|| AccountApiError::InternalError("Customer balance not set".to_string()))?,
            order_balance: self
                .order_balance
                .ok_or_else(|| AccountApiError::InternalError("Customer order balance not set".to_string()))?,
            orders: self.orders.ok_or_else(|| AccountApiError::InternalError("Customer orders not set".to_string()))?,
            settlements: self
                .settlements
                .ok_or_else(|| AccountApiError::InternalError("Customer settlements not set".to_string()))?,
        };
        Ok(history)
    }
}

impl CustomerHistory {
    pub fn builder(customer_id: String) -> CustomerHistoryBuilder {
        CustomerHistoryBuilder { customer_id, ..Default::default() }
    }
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
