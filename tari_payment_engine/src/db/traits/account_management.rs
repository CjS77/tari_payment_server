use tari_common_types::tari_address::TariAddress;

use crate::db_types::{Order, OrderId, Payment, UserAccount};

/// The `AccountManagement` trait defines behaviour for managing accounts.
/// An account is a record that associates one or more Tari wallets (via their address) and their associated
/// payments with a set of orders from the merchant.
///
/// The [`PaymentGatewayDatabase`] trait handles the actual machinery of matching Tari addresses with merchant accounts
/// and orders. `AccountManagement` provides methods for querying information about these accounts.
#[allow(async_fn_in_trait)]
pub trait AccountManagement {
    type Error: std::error::Error;
    /// Fetches the user account associated with the given account id. If no account exists, `None` is returned.
    async fn fetch_user_account(&self, account_id: i64) -> Result<Option<UserAccount>, Self::Error>;

    /// Fetches the user account for the given order id. A user account must have already been created for this account.
    /// If no account is found, `None` will be returned.
    ///
    /// Alternatively, you can search through the memo fields of payments to find a matching order id by calling
    /// [`search_for_user_account_by_memo`].
    async fn fetch_user_account_for_order(&self, order_id: &OrderId) -> Result<Option<UserAccount>, Self::Error>;

    async fn fetch_user_account_for_customer_id(&self, customer_id: &str) -> Result<Option<UserAccount>, Self::Error>;
    async fn fetch_user_account_for_address(&self, address: &TariAddress) -> Result<Option<UserAccount>, Self::Error>;

    async fn fetch_orders_for_account(&self, account_id: i64) -> Result<Vec<Order>, Self::Error>;

    async fn fetch_order_by_order_id(&self, order_id: &OrderId) -> Result<Option<Order>, Self::Error>;

    async fn fetch_payments_for_address(&self, address: &TariAddress) -> Result<Vec<Payment>, Self::Error>;
}
