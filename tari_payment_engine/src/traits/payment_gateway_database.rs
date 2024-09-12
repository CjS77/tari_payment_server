use chrono::Duration;
use tari_common_types::tari_address::TariAddress;
use thiserror::Error;
use tpg_common::MicroTari;

use crate::{
    db_types::{CreditNote, NewOrder, NewPayment, Order, OrderId, OrderStatusType, Payment, TransferStatus},
    order_objects::OrderChanged,
    traits::{
        data_objects::{ExpiryResult, MultiAccountPayment, OrderMovedResult},
        AccountApiError,
        AccountManagement,
    },
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

    /// Claim the order, identified by `order_id` for the given wallet `address`.
    ///
    /// The order must currently be `Unclaimed`.
    ///
    /// The customer id in the order will be associated with the address as part of the claim.
    /// Returns the updated order record.
    async fn claim_order(&self, order_id: &OrderId, address: &TariAddress) -> Result<Order, PaymentGatewayError>;

    /// Attempt to automatically claim the order, by looking for any addresses that are already associated
    /// with the customer id in the order. If there are multiple addresses, the most recent one is used.
    ///
    /// The order status must be `Unclaimed`, and will be set to `New` if the claim is successful.
    async fn auto_claim_order(&self, order: &Order) -> Result<Option<(TariAddress, Order)>, PaymentGatewayError>;

    /// Takes a new order, and in a single atomic transaction, stores the order in the database.
    /// This call is idempotent
    /// Returns true if the order was inserted, or false if it already existed.
    async fn insert_order(&self, order: NewOrder) -> Result<(Order, bool), PaymentGatewayError>;

    /// Takes a new payment, and in a single atomic transaction,
    /// * calls `save_payment` to store the payment in the database. If the payment already exists, nothing further is
    ///   done.
    /// * The payment is marked as `Unconfirmed`
    ///
    /// Returns the newly created Payment record.
    async fn process_new_payment(&self, payment: NewPayment) -> Result<Payment, PaymentGatewayError>;

    /// Fetches all pending payments for the given address.
    async fn fetch_pending_payments_for_address(
        &self,
        address: &TariAddress,
    ) -> Result<Vec<Payment>, PaymentGatewayError>;

    /// Creates a new credit note for a customer id
    /// * Stores the payment in the database. If the payment already exists, nothing further is   done.
    /// * The payment is marked as `Confirmed`
    /// * The payment type is set to `Manual`
    ///
    /// A random wallet address is generated for the credit note, and the payment is linked to the customer id.
    ///
    /// Returns the payment record for the credit note.
    async fn process_credit_note_for_customer(&self, note: CreditNote) -> Result<Payment, PaymentGatewayError>;

    /// Checks whether any orders associated with the given address can be fulfilled.
    async fn fetch_payable_orders_for_address(&self, address: &TariAddress) -> Result<Vec<Order>, PaymentGatewayError>;

    /// Tries to pay for an order using any addresses associated with the customer id attached to this order.
    /// If you've claimed an order, or otherwise know which address you want to pay from, use
    /// [`try_pay_orders_from_address`] instead.
    async fn try_pay_order(&self, order: &Order) -> Result<MultiAccountPayment, PaymentGatewayError>;

    /// Tries to fulfil the orders using the address as payment source.
    ///
    /// This method will not try and use other addresses that are also linked to the customer ids in the order list.
    async fn try_pay_orders_from_address(
        &self,
        address: &TariAddress,
        orders: &[&Order],
    ) -> Result<MultiAccountPayment, PaymentGatewayError>;

    /// Updates the payment status for the given transaction id. This is typically called to transition a payment from
    /// `Unconfirmed` to `Confirmed` or `Cancelled`.
    ///
    /// If the transaction was not "Received", an error is returned.
    /// If the status is unchanged, then nothing is changed, and `None` is returned.
    ///
    /// If the status is changed, the account id corresponding to the transaction is returned.
    async fn update_payment_status(&self, tx_id: &str, status: TransferStatus) -> Result<Payment, PaymentGatewayError>;

    /// Fetches the payment for the given transaction id.
    async fn fetch_payment_by_tx_id(&self, tx_id: &str) -> Result<Payment, PaymentGatewayError>;

    /// A manual order status transition from `New` to `Paid` status.
    /// This method is called by the default implementation of [`modify_status_for_order`] when the new status is
    /// `Paid`. When this happens, the following side effects occur:
    ///
    /// * A credit note for the `total_price` is created,
    /// * The `process_new_payment` flow is triggered, which will cause the order to be fulfilled and the status updated
    ///   to `Paid`.
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
    /// * An entry is added to the audit log.
    async fn reset_order(&self, order: &OrderId) -> Result<OrderChanged, PaymentGatewayError>;

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
    ) -> Result<OrderMovedResult, PaymentGatewayError>;

    /// Changes the memo field for an order.
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
    /// - An entry in the audit log is made.
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

    /// Marks unapid and unclaimed orders as expired.
    ///
    /// Any orders that have not been _updated_ (based on the `updated_at` field) for longer than the given duration
    /// will be marked as `Expired`.
    ///
    /// Typical values for the `unclaimed_limit` are 2 hours, and for the `unpaid_limit` are 48 hours.
    ///
    /// The result is a list of orders that were expired.
    async fn expire_old_orders(
        &self,
        unclaimed_limit: Duration,
        unpaid_limit: Duration,
    ) -> Result<ExpiryResult, PaymentGatewayError>;

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
    OrderAlreadyExists(OrderId),
    #[error("Cannot insert payment, since it already exists with txid {0}")]
    PaymentAlreadyExists(String),
    #[error("{0}")]
    AccountError(#[from] AccountApiError),
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
    #[error("The requested payment update would result in a no-op.")]
    PaymentModificationNoOp,
    #[error("The requested order (internal id {0}) does not exist")]
    OrderIdNotFound(i64),
    #[error("The requested order {0} does not exist")]
    OrderNotFound(OrderId),
    #[error("{0} are not supported yet")]
    UnsupportedAction(String),
    #[error("Cannot claim order because the signature is invalid.")]
    InvalidSignature,
    #[error("The requested payment does not exist for txid {0}")]
    PaymentNotFound(String),
}

impl From<sqlx::Error> for PaymentGatewayError {
    fn from(e: sqlx::Error) -> Self {
        PaymentGatewayError::DatabaseError(e.to_string())
    }
}
