use tari_common_types::tari_address::TariAddress;
use thiserror::Error;

use crate::{
    db_types::{Order, OrderId, Payment, UserAccount},
    order_objects::OrderQueryFilter,
};

#[derive(Debug, Clone, Error)]
pub enum AccountApiError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("User error constructing query: {0}")]
    QueryError(String),
}

impl From<sqlx::Error> for AccountApiError {
    fn from(e: sqlx::Error) -> Self {
        AccountApiError::DatabaseError(e.to_string())
    }
}

/// The `AccountManagement` trait defines behaviour for managing accounts.
/// An account is a record that associates one or more Tari wallets (via their address) and their associated
/// payments with a set of orders from the merchant.
///
/// The [`PaymentGatewayDatabase`] trait handles the actual machinery of matching Tari addresses with merchant accounts
/// and orders. `AccountManagement` provides methods for querying information about these accounts.
#[allow(async_fn_in_trait)]
pub trait AccountManagement {
    /// Fetches the user account associated with the given account id. If no account exists, `None` is returned.
    async fn fetch_user_account(&self, account_id: i64) -> Result<Option<UserAccount>, AccountApiError>;

    /// Fetches the user account for the given order id. A user account must have already been created for this account.
    /// If no account is found, `None` will be returned.
    ///
    /// Alternatively, you can search through the memo fields of payments to find a matching order id by calling
    /// [`search_for_user_account_by_memo`].
    async fn fetch_user_account_for_order(&self, order_id: &OrderId) -> Result<Option<UserAccount>, AccountApiError>;

    async fn fetch_user_account_for_customer_id(
        &self,
        customer_id: &str,
    ) -> Result<Option<UserAccount>, AccountApiError>;
    async fn fetch_user_account_for_address(
        &self,
        address: &TariAddress,
    ) -> Result<Option<UserAccount>, AccountApiError>;

    async fn fetch_orders_for_account(&self, account_id: i64) -> Result<Vec<Order>, AccountApiError>;

    async fn fetch_order_by_order_id(&self, order_id: &OrderId) -> Result<Option<Order>, AccountApiError>;

    async fn fetch_payments_for_address(&self, address: &TariAddress) -> Result<Vec<Payment>, AccountApiError>;

    async fn search_orders(&self, query: OrderQueryFilter) -> Result<Vec<Order>, AccountApiError>;
}
