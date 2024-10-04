//! Unifies API for accessing accounts.

use std::fmt::Debug;

use log::*;
use tari_common_types::tari_address::TariAddress;

use crate::{
    db_types::{AddressBalance, CustomerBalance, CustomerOrders, Order, OrderId, Payment},
    order_objects::{OrderQueryFilter, OrderResult},
    tpe_api::{
        account_objects::{AddressHistory, CustomerHistory, Pagination},
        payment_objects::PaymentsResult,
    },
    traits::{AccountApiError, AccountManagement},
};

/// The `AccountApi` provides a unified API for accessing accounts.
pub struct AccountApi<B> {
    db: B,
}

impl<B: Debug> Debug for AccountApi<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AccountApi ({:?})", self.db)
    }
}

impl<B> AccountApi<B>
where B: AccountManagement
{
    pub fn new(db: B) -> Self {
        Self { db }
    }

    pub async fn fetch_order_by_order_id(&self, order_id: &OrderId) -> Result<Option<Order>, AccountApiError> {
        self.db.fetch_order_by_order_id(order_id).await
    }

    /// Fetches all orders associated with the given Tari address, and wraps them in an `OrderResult`, which includes
    /// the metadata of the address and the sum of the orders' values.
    pub async fn orders_for_address(&self, address: &TariAddress) -> Result<OrderResult, AccountApiError> {
        let orders = self.db.fetch_orders_for_address(address).await?;
        let total_orders = orders.iter().map(|o| o.total_price).sum();
        let result = OrderResult { address: address.into(), total_orders, orders };
        Ok(result)
    }

    /// Fetches all payments associated with the given Tari address, and wraps them in a `PaymentsResult`, which
    /// includes the metadata of the address and the sum of the payments' values.
    pub async fn payments_for_address(&self, address: &TariAddress) -> Result<PaymentsResult, AccountApiError> {
        let payments = self.db.fetch_payments_for_address(address).await?;
        trace!("Payments for address: {:?}", payments);
        let total_payments = payments.iter().map(|p| p.amount).sum();
        trace!("Total payments for address: {:?}", total_payments);
        Ok(PaymentsResult { address: address.clone().into(), total_payments, payments })
    }

    /// Returns the consolidated account history for the given address, if it exists.
    /// This includes all orders and payments associated with the address.
    /// If the address does not exist, `None` is returned.
    pub async fn history_for_address(&self, address: &TariAddress) -> Result<AddressHistory, AccountApiError> {
        self.db.history_for_address(address).await
    }

    /// Returns the consolidated account history for the given customer id, if it exists.
    /// This includes all orders and payments associated with the account.
    /// If the account does not exist, `None` is returned.
    pub async fn history_for_customer(&self, customer_id: &str) -> Result<CustomerHistory, AccountApiError> {
        self.db.history_for_customer(customer_id).await
    }

    pub async fn search_orders(&self, query: OrderQueryFilter) -> Result<Vec<Order>, AccountApiError> {
        self.db.search_orders(query).await.map_err(|e| AccountApiError::DatabaseError(e.to_string()))
    }

    pub async fn creditors(&self) -> Result<Vec<CustomerOrders>, AccountApiError> {
        let creditors = self.db.creditors().await?;
        info!("📋️ Creditors result: {} customers have outstanding orders", creditors.len());
        Ok(creditors)
    }

    pub async fn fetch_customer_ids(&self, pagination: &Pagination) -> Result<Vec<String>, AccountApiError> {
        self.db.fetch_customer_ids(pagination).await
    }

    pub async fn fetch_addresses(&self, pagination: &Pagination) -> Result<Vec<TariAddress>, AccountApiError> {
        self.db.fetch_addresses(pagination).await
    }

    pub async fn fetch_address_balance(&self, address: &TariAddress) -> Result<AddressBalance, AccountApiError> {
        self.db.fetch_address_balance(address).await
    }

    pub async fn fetch_customer_balance(&self, customer_id: &str) -> Result<CustomerBalance, AccountApiError> {
        self.db.fetch_customer_balance(customer_id).await
    }

    pub async fn fetch_payments_for_order(&self, order_id: &OrderId) -> Result<Vec<Payment>, AccountApiError> {
        self.db.fetch_payments_for_order(order_id).await
    }
}
