use thiserror::Error;

use crate::{
    db_types::{NewOrder, NewPayment, Order, OrderId, OrderUpdate, Payment, TransferStatus},
    traits::AccountApiError,
};

/// This trait defines the highest level of behaviour for backends supporting the Tari Payment Engine.
///
/// This behaviour includes:
/// * Fetching and creating accounts to track and associates payments with orders.
/// * Handling incoming payment events
/// * Handling incoming order events
/// * Order fulfilment flow management
#[allow(async_fn_in_trait)]
pub trait PaymentGatewayDatabase: Clone {
    /// The URL of the database
    fn url(&self) -> &str;

    /// Fetches the user account for the given order.
    ///
    /// If the account does not exist, one is created and the given customer id and/or public key is linked to the
    /// account.
    async fn fetch_or_create_account_for_order(&self, order: &NewOrder) -> Result<i64, PaymentGatewayError>;

    /// Fetches the user account for the given payment.
    ///
    /// If the account does not exist, one is created and the given public key and (if present) customer id is linked to
    /// the account.
    async fn fetch_or_create_account_for_payment(&self, payment: &Payment) -> Result<i64, PaymentGatewayError>;

    /// Takes a new order, and in a single atomic transaction,
    /// * calls `save_new_order` to store the order in the database. If the order already exists, nothing further is
    ///   done.
    /// * creates a new account for the customer if one does not already exist
    /// * Updates the total orders value for the account
    ///
    /// Returns the account id for the customer.
    async fn process_new_order_for_customer(&self, order: NewOrder) -> Result<i64, PaymentGatewayError>;

    /// Takes a new payment, and in a single atomic transaction,
    /// * calls `save_payment` to store the payment in the database. If the payment already exists, nothing further is
    ///   done.
    /// * The payment is marked as `Unconfirmed`
    /// * creates a new account for the public key if one does not already exist
    /// Returns the account id for the public key.
    async fn process_new_payment_for_pubkey(&self, payment: NewPayment) -> Result<i64, PaymentGatewayError>;

    /// Checks whether any orders associated with the given account id can be fulfilled.
    /// If no orders can be fulfilled, an empty vector is returned.
    async fn fetch_payable_orders(&self, account_id: i64) -> Result<Vec<Order>, PaymentGatewayError>;

    /// Tries to fulfil the list of orders given from the given account.
    ///
    /// Any order that has enough credit in the account
    /// * Will be marked as Paid
    /// * Returned in the result vector.
    async fn try_pay_orders(&self, account_id: i64, orders: &[Order]) -> Result<Vec<Order>, PaymentGatewayError>;

    /// Updates the payment status for the given transaction id. This is typically called to transition a payment from
    /// `Unconfirmed` to `Confirmed` or `Cancelled`.
    ///
    /// If the transaction was not "Received", an error is returned.
    /// If the status is unchanged, then nothing is changed, and `None` is returned.
    ///
    /// If the status is changed, the account id corresponding to the transaction is returned.
    async fn update_payment_status(
        &self,
        tx_id: &str,
        status: TransferStatus,
    ) -> Result<Option<i64>, PaymentGatewayError>;

    /// Updates the order details for the given order id. Not all fields are permitted to be updated, so
    /// `OrderUpdate` only exposes those that can be changed.
    async fn update_order(&self, id: &OrderId, update: OrderUpdate) -> Result<(), PaymentGatewayError>;

    /// Closes the database connection.
    async fn close(&mut self) -> Result<(), PaymentGatewayError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Error)]
pub enum PaymentGatewayError {
    #[error("We have an internal database engine (configuration/uptime etc.) : {0}")]
    DatabaseError(String),
    #[error("Cannot insert order, since it already exists with id {0}")]
    OrderAlreadyExists(i64),
    #[error("Cannot insert payment, since it already exists with txid {0}")]
    PaymentAlreadyExists(String),
    #[error("{0}")]
    UserAccountError(#[from] AccountApiError),
    #[error("The requested account id {0} does not exist")]
    AccountNotFound(i64),
    #[error("Illegal payment status change. {0}")]
    PaymentStatusUpdateError(String),
    #[error("Account not linked. {0}")]
    AccountNotLinkedWithTransaction(String),
}

impl From<sqlx::Error> for PaymentGatewayError {
    fn from(e: sqlx::Error) -> Self {
        PaymentGatewayError::DatabaseError(e.to_string())
    }
}
