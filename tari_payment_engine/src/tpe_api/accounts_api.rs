//! Unifies API for accessing accounts.

use std::fmt::Debug;

use log::trace;
use tari_common_types::tari_address::TariAddress;

use crate::{
    db_types::{Order, OrderId, UserAccount},
    order_objects::{OrderQueryFilter, OrderResult},
    tpe_api::{account_objects::FullAccount, payment_objects::PaymentsResult},
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

    /// Fetches the user account for the given account id. If no account exists, `None` is returned.
    pub async fn account_by_id(&self, account_id: i64) -> Result<Option<UserAccount>, AccountApiError> {
        self.db.fetch_user_account(account_id).await
    }

    /// Fetches the user account for the given Tari address.
    pub async fn account_by_address(&self, address: &TariAddress) -> Result<Option<UserAccount>, AccountApiError> {
        self.db.fetch_user_account_for_address(address).await
    }

    pub async fn fetch_order_by_order_id(&self, order_id: &OrderId) -> Result<Option<Order>, AccountApiError> {
        self.db.fetch_order_by_order_id(order_id).await
    }

    /// Fetches all orders associated with the given Tari address, and wraps them in an `OrderResult`, which includes
    /// the metadata of the address and the sum of the orders' values.
    pub async fn orders_for_address(&self, address: &TariAddress) -> Result<Option<OrderResult>, AccountApiError> {
        let mut result = OrderResult { address: address.clone(), total_orders: 0.into(), orders: vec![] };
        match self.account_by_address(address).await {
            Ok(Some(acc)) => {
                result.total_orders = acc.total_orders;
                let orders = self.db.fetch_orders_for_account(acc.id).await?;
                result.orders = orders;
                Ok(Some(result))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
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
    pub async fn history_for_address(&self, address: &TariAddress) -> Result<Option<FullAccount>, AccountApiError> {
        self.db.history_for_address(address).await
    }

    /// Returns the consolidated account history for the given account id, if it exists.
    /// This includes all orders and payments associated with the account.
    /// If the account does not exist, `None` is returned.
    pub async fn history_for_id(&self, account_id: i64) -> Result<Option<FullAccount>, AccountApiError> {
        self.db.history_for_id(account_id).await
    }

    pub async fn search_orders(
        &self,
        query: OrderQueryFilter,
        only_for: Option<TariAddress>,
    ) -> Result<Vec<Order>, AccountApiError> {
        self.db.search_orders(query, only_for).await.map_err(|e| AccountApiError::DatabaseError(e.to_string()))
    }
}
