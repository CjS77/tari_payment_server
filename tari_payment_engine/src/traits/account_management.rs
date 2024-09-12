use tari_common_types::tari_address::TariAddress;
use thiserror::Error;

use crate::{
    db_types::{AddressBalance, CustomerBalance, CustomerOrderBalance, CustomerOrders, Order, OrderId, Payment},
    order_objects::OrderQueryFilter,
    tpe_api::account_objects::{AddressHistory, CustomerHistory, Pagination},
};

#[derive(Debug, Clone, Error)]
pub enum AccountApiError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("User error constructing query: {0}")]
    QueryError(String),
    #[error("The requested order does not exist: {0}")]
    OrderDoesNotExist(OrderId),
    #[error("The requested address does not exist in the database: {0}")]
    AddressDoesNotExists(TariAddress),
    #[error("Insufficient funds to complete the transaction")]
    InsufficientFunds,
    #[error("Cannot uniquely determine the account for the address-customer pair")]
    AmbiguousAccounts(AmbiguousAccountInfo),
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl AccountApiError {
    pub fn dne(oid: OrderId) -> Self {
        AccountApiError::OrderDoesNotExist(oid)
    }

    pub fn ambiguous(
        address: TariAddress,
        address_account_ids: Vec<i64>,
        customer_id: String,
        customer_account_id: i64,
    ) -> Self {
        AccountApiError::AmbiguousAccounts(AmbiguousAccountInfo {
            address,
            address_account_ids,
            customer_id,
            customer_account_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AmbiguousAccountInfo {
    pub address: TariAddress,
    pub address_account_ids: Vec<i64>,
    pub customer_id: String,
    pub customer_account_id: i64,
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
/// The [`crate::traits::PaymentGatewayDatabase`] trait handles the actual machinery of matching Tari addresses with
/// merchant accounts and orders. `AccountManagement` provides methods for querying information about these accounts.
#[allow(async_fn_in_trait)]
pub trait AccountManagement {
    async fn fetch_orders_for_address(&self, acaddress: &TariAddress) -> Result<Vec<Order>, AccountApiError>;

    async fn fetch_order_by_order_id(&self, order_id: &OrderId) -> Result<Option<Order>, AccountApiError>;

    async fn fetch_payments_for_address(&self, address: &TariAddress) -> Result<Vec<Payment>, AccountApiError>;

    /// Returns the consolidated account history for the given address, if it exists.
    async fn history_for_address(&self, address: &TariAddress) -> Result<AddressHistory, AccountApiError>;

    /// Returns the consolidated account history for the given customer id, if it exists.
    async fn history_for_customer(&self, customer_id: &str) -> Result<CustomerHistory, AccountApiError>;

    async fn search_orders(&self, query: OrderQueryFilter) -> Result<Vec<Order>, AccountApiError>;

    async fn creditors(&self) -> Result<Vec<CustomerOrders>, AccountApiError>;

    async fn fetch_customer_ids(&self, pagination: &Pagination) -> Result<Vec<String>, AccountApiError>;

    async fn fetch_addresses(&self, pagination: &Pagination) -> Result<Vec<TariAddress>, AccountApiError>;

    /// Fetches the balance for the given address
    ///
    /// This includes all payments received, both pending and confirmed tallied against all orders paid for by the
    /// address.
    async fn fetch_address_balance(&self, address: &TariAddress) -> Result<AddressBalance, AccountApiError>;

    /// Fetches the balance for the given customer
    ///
    /// This method fetches all balances for all wallets associated with the customer id.
    ///
    /// It's possible that a wallet is associated with multiple customer ids, in which case the balances will be
    /// double counted, so don't rely on the result of this method for broad accounting/auditing purposes.
    async fn fetch_customer_balance(&self, customer_id: &str) -> Result<CustomerBalance, AccountApiError>;

    /// Fetches the state of orders made with respect to this customer id.
    ///
    /// This includes the total paid prders, current orders, and expired and cancelled orders.
    async fn fetch_customer_order_balance(&self, customer_id: &str) -> Result<CustomerOrderBalance, AccountApiError>;
}
