use thiserror::Error;

use crate::{
    db_types::{CreditNote, MicroTari, NewOrder, NewPayment, Order, OrderId, OrderStatusType, Payment, TransferStatus},
    order_objects::OrderChanged,
    traits::{AccountApiError, AccountManagement},
};

/// This trait defines the highest level of behaviour for backends supporting the Tari Payment Engine.
///
/// This behaviour includes:
/// * Fetching and creating accounts to track and associates payments with orders.
/// * Handling incoming payment events
/// * Handling incoming order events
/// * Order fulfilment flow management
#[allow(async_fn_in_trait)]
pub trait PaymentGatewayDatabase: Clone + AccountManagement {
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

    /// Creates a new credit note for a customer id
    /// * Stores the payment in the database. If the payment already exists, nothing further is   done.
    /// * The payment is marked as `Confirmed`
    /// * The payment type is set to `Manual`
    /// * creates a new account for the customer id if one does not already exist
    /// Returns the account id for the customer id.
    async fn process_credit_note_for_customer(&self, note: CreditNote) -> Result<i64, PaymentGatewayError>;

    /// Checks whether any orders associated with the given account id can be fulfilled.
    /// If no orders can be fulfilled, an empty vector is returned.
    async fn fetch_payable_orders(&self, account_id: i64) -> Result<Vec<Order>, PaymentGatewayError>;

    /// Tries to fulfil the list of orders given from the given account.
    ///
    /// Any order that has enough credit in the account
    /// * Will be marked as Paid
    /// * Returned in the result vector.
    /// * Will fire an OnOrderPaid hook if one is set.
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

    /// A manual order status transition from `New` to `Paid` status.
    /// This method is called by the default implementation of [`modify_status_for_order`] when the new status is
    /// `Paid`. When this happens, the following side effects occur:
    ///
    /// * A credit note for the `total_price` is created,
    /// * The `process_new_payment` flow is triggered, which will cause the order to be fulfilled and the status updated
    ///   to `Paid`.(TODO at API level)
    async fn mark_new_order_as_paid(&self, order: Order, reason: &str) -> Result<Order, PaymentGatewayError>;

    /// A manual order status transition from `New` to `Expired` or `Cancelled` status.
    ///
    /// This method is called by the default implementation of [`modify_status_for_order`] when the new status
    /// is `Expired`, or `Cancelled`.
    ///
    /// The side effects for expiring or cancelling an order are the same. The only difference is that Expired orders
    /// are triggered automatically based on time, whereas cancelling an order is triggered by the user or an admin.
    ///
    /// * The order status is updated in the database.
    /// * The total orders for the account are updated.
    /// * The `OnOrderModified` event is triggered. (TODO at API level)
    /// * An audit log entry is made.
    async fn cancel_or_expire_order(
        &self,
        order: &OrderId,
        new_status: OrderStatusType,
        reason: &str,
    ) -> Result<Order, PaymentGatewayError>;

    /// Manually reset an order from `Expired` or `Cancelled` status to `New` status.
    ///
    /// This method is called by the default implementation of [`modify_status_for_order`] when the new status
    /// is `New`. This is often done as a follow-up step to changing the customer id for an order.
    ///
    /// The side effects for resetting an order are the same for both Expired and Cancelled orders.
    /// The effect is as if a new order comes in with the given details.
    ///
    /// * The order status is updated in the database.
    /// * The [`process_order`] flow is triggered.
    /// * An `OnOrderModified` event is triggered. (TODO at API level)
    /// * A `NewOrder` event is triggered. (TODO at API level)
    /// * An entry is added to the audit log.
    async fn reset_order(&self, order: Order) -> Result<Order, PaymentGatewayError>;

    /// Change the customer id for the given `order_id`. This function has several side effects:
    /// - The `customer_id` field of the order is updated in the database.
    /// - The total orders for the old and new customer are updated.
    /// - If the order is fulfillable with existing payments in the new account, the fulfillment flow is triggered.
    /// - If the new customer does not exist, a new one is created.
    /// - If the order status was `Expired`, or `Cancelled`, it is **not** automatically reset to `New`. The admin must
    ///   follow up with a "change status" call to reset the order.
    /// - The `OnOrderModified` event is triggered.
    ///
    /// ## Returns:
    /// - The old and new account ids.
    ///
    ///
    /// ## Failure modes:
    /// - If the order does not exist, an error is returned.
    /// - If the order status is already `Paid`, an error is returned.
    async fn modify_customer_id_for_order(
        &self,
        order_id: &OrderId,
        new_customer_id: &str,
    ) -> Result<(i64, i64), PaymentGatewayError>;

    /// Changes the memo field for an order.
    ///
    /// This function has the following side effects.
    /// - The `OnOrderModified` event is triggered. TODO (at API level)
    ///
    /// Changing the memo does not trigger any other flows, does not affect
    /// the order status, and does not affect order fulfillment.
    ///
    /// ## Returns:
    /// The modified order
    async fn modify_memo_for_order(&self, order_id: &OrderId, new_memo: &str) -> Result<Order, PaymentGatewayError>;

    /// Changes the total price for an order.
    ///
    /// To return successfully, the order must exist, and have `New` status.
    /// This function has several side effects:
    /// - The `total_price` field of the order is updated in the database.
    /// - The total orders for the account are updated.
    /// - If the order is now fulfillable with existing payments in the account, the fulfillment flow is triggered (TODO
    ///   at API level).
    /// - An entry in the audit log is made.
    /// - The `OnOrderModified` event is triggered.  (TODO at API level)
    ///
    /// ## Failure modes:
    /// - If the order does not exist.
    /// - If the order status was `Expired`, or `Cancelled`.
    /// - If the order status is `Paid`. To handle refunds or post-payment adjustments, use the `credit_note` function.
    ///
    /// ## Returns
    /// The modified order
    async fn modify_total_price_for_order(
        &self,
        order_id: &OrderId,
        new_total_price: MicroTari,
    ) -> Result<OrderChanged, PaymentGatewayError>;

    /// Since only XTR is supported currently, this method will always return an error.
    async fn modify_currency_for_order(
        &self,
        _order_id: &OrderId,
        _new_currency: &str,
    ) -> Result<Order, PaymentGatewayError> {
        Err(PaymentGatewayError::UnsupportedAction("Multiple currencies".to_string()))
    }

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
    #[error("The requested account does not exist (even though it should): {0}")]
    AccountShouldExistForOrder(OrderId),
    #[error("Illegal payment status change. {0}")]
    PaymentStatusUpdateError(String),
    #[error("Account not linked. {0}")]
    AccountNotLinkedWithTransaction(String),
    #[error("The requested order change would result in a no-op.")]
    OrderModificationNoOp,
    #[error("The requested order change is forbidden.")]
    OrderModificationForbidden,
    #[error("The requested order (internal id {0}) does not exist")]
    OrderIdNotFound(i64),
    #[error("The requested order {0} does not exist")]
    OrderNotFound(OrderId),
    #[error("{0} are not supported yet")]
    UnsupportedAction(String),
}

impl From<sqlx::Error> for PaymentGatewayError {
    fn from(e: sqlx::Error) -> Self {
        PaymentGatewayError::DatabaseError(e.to_string())
    }
}
