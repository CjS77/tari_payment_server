use std::fmt::Debug;

use log::*;

use crate::{
    db_types::{NewOrder, NewPayment, Order, TransferStatus},
    events::{EventProducers, OrderPaidEvent},
    traits::{PaymentGatewayDatabase, PaymentGatewayError},
};
use crate::db_types::{MicroTari, OrderId, OrderStatusType};
use crate::db_types::OrderStatusType::{Cancelled, Expired, New, Paid};
use crate::helpers::create_dummy_address_for_cust_id;
use crate::traits::AccountApiError;

/// `OrderFlowApi` is the primary API for handling order and payment flows in response to merchant order events and
/// wallet payment events.
pub struct OrderFlowApi<B> {
    db: B,
    producers: EventProducers,
}

impl<B> Debug for OrderFlowApi<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OrderManagerApi")
    }
}

impl<B> OrderFlowApi<B> {
    pub fn new(db: B, producers: EventProducers) -> Self {
        Self { db, producers }
    }
}

impl<B> OrderFlowApi<B>
where B: PaymentGatewayDatabase
{
    /// Submit a new order to the order manager.
    ///
    /// This should be a brand-new order. If the order already exists, the order manager will return an error.
    /// To change details about an order, you should use the [`Self::update_order`] method.
    ///
    /// After the order is added, all the orders for the account are checked to see if any can be marked as paid.
    /// If any orders are marked as paid, they are returned.
    pub async fn process_new_order(&self, order: NewOrder) -> Result<Vec<Order>, PaymentGatewayError> {
        let account_id = self.db.process_new_order_for_customer(order.clone()).await?;
        // TODO NewOrderEvent hook handler
        let payable = self.db.fetch_payable_orders(account_id).await?;
        let paid_orders = self.db.try_pay_orders(account_id, &payable).await?;
        self.call_order_paid_hook(&paid_orders).await;
        debug!(
            "🔄️📦️ Order [{}] processing complete. {} orders are paid for account #{account_id}",
            order.order_id,
            payable.len()
        );
        Ok(paid_orders)
    }

    async fn call_order_paid_hook(&self, paid_orders: &[Order]) {
        for emitter in &self.producers.order_paid_producer {
            debug!("🔄️📦️ Notifying order paid hook subscribers");
            for order in paid_orders {
                let event = OrderPaidEvent { order: order.clone() };
                emitter.publish_event(event).await;
            }
        }
    }

    /// Submit a new payment to the order manager.
    ///
    /// This should be a brand-new payment. If the payment already exists, the order manager will return an error.
    /// To change the status of a payment, you should use the [`Self::confirm_payment`] or [`Self::cancel_payment`]
    /// methods.
    ///
    /// After the payment is added, all the orders for the account are checked to see if any can be marked as paid.
    /// If any orders are marked as paid, they are returned.
    pub async fn process_new_payment(&self, payment: NewPayment) -> Result<Vec<Order>, PaymentGatewayError> {
        let txid = payment.txid.clone();
        let account_id = self.db.process_new_payment_for_pubkey(payment.clone()).await?;
        trace!("🔄️💰️ Payment [{txid}] for account #{account_id} processed.");
        // todo insert hook
        let payable = self.db.fetch_payable_orders(account_id).await?;
        trace!("🔄️💰️ {} fulfillable orders fetched for account #{account_id}", payable.len());
        let paid_orders = self.db.try_pay_orders(account_id, &payable).await?;
        self.call_order_paid_hook(&paid_orders).await;
        debug!(
            "🔄️💰️ Payment [{txid}] processing complete. {} orders are paid for account #{account_id}",
            payable.len()
        );
        Ok(paid_orders)
    }

    pub async fn issue_credit_note(&self, cust_id: &str, amount: MicroTari, memo: &str) -> Result<(), PaymentGatewayError> {
        let mut account = self.db.fetch_user_account_for_customer_id(cust_id).await?;
        if account.is_none() {
            info!("🔄️💰️ There is no Tari Address associated with customer id {} yet. In order to continue, I am \
                creating a dummy Tari address associated with this customer with which to apply the credit.", cust_id);
            let address = create_dummy_address_for_cust_id(cust_id);
            let txid = format!("credit_note_{cust_id}");
            let mut payment = NewPayment::new(address, amount, txid);
            // payment.with_memo(format!("Credit note: {memo}"));
            // let acc_id = self.db.prfetch_or_create_account_for_payment(&payment).await?;
            // let new_account = self.db.fetch_user_account(acc_id).await?
            //     .ok_or_else(|| {
            //         error!("🔄️💰️ Account #{acc_id} was not found straight after creating it. This is a data race bug.");
            //         PaymentGatewayError::AccountNotFound(acc_id)
            //     })?;
            // account = Some(new_account);
        }
        //let credit_note = self.db.issue_credit_note(account_id, amount, memo).await?;
        Ok(())
    }

    /// Update the status of a payment to "Confirmed". This happens when a transaction in the blockchain is deep enough
    /// in the chain that a re-org and invalidation of the payment is unlikely.
    pub async fn confirm_payment(&self, txid: String) -> Result<Vec<Order>, PaymentGatewayError> {
        trace!("🔄️✅️ Payment {txid} is being marked as confirmed");
        let account_id = self.db.update_payment_status(&txid, TransferStatus::Confirmed).await?;
        let paid_orders = match account_id {
            Some(acc_id) => {
                let payable = self.db.fetch_payable_orders(acc_id).await?;
                trace!("🔄️✅️ {} fulfillable orders fetched for account #{acc_id}", payable.len());
                let paid_orders = self.db.try_pay_orders(acc_id, &payable).await?;
                debug!("🔄️✅️ [{txid}] confirmed. {} orders are paid for account #{acc_id}", payable.len());
                self.call_order_paid_hook(&paid_orders).await;
                paid_orders
            },
            None => {
                error!("🔄️✅️ [{txid}] confirmed, but it is not linked to any account!");
                Vec::new()
            },
        };
        Ok(paid_orders)
    }

    /// Mark a payment as cancelled and update orders and accounts as necessary.
    pub async fn cancel_payment(&self, txid: String) -> Result<(), PaymentGatewayError> {
        trace!("🔄️❌️ Payment {txid} is being marked as cancelled");
        self.db.update_payment_status(&txid, TransferStatus::Cancelled).await?;
        Ok(())
    }

    /// Changes the status of an order.
    ///
    /// This function has several side effects, depending on the current order status and the new order status. The
    /// results are summarised in this table, with detailed notes provided in the subsequent sections.
    ///
    /// | From \ To | New  | Expired | Cancelled | Paid |
    /// |-----------|------|---------|-----------|------|
    /// | New       | Err  | 1       | 1         | 3    |
    /// | Expired   | 2    | Err     | Err       | Err  |
    /// | Cancelled | 2    | Err     | Err       | Err  |
    /// | Paid      | Err  | Err     | Err       | Err  |
    ///
    /// ### (1) Changing from `New` to `Expired` or `Cancelled`
    ///
    /// The side effects for expiring or cancelling an order are the same. The only difference is that Expired orders
    /// are triggered automatically based on time, whereas cancelling an order is triggered by the user or an admin.
    ///
    /// * The order status is updated in the database.
    /// * The total orders for the account are updated.
    /// * The `OnOrderModified` event is triggered.
    /// * An audit log entry is made.
    ///
    /// ### (2) Changing from `Expired` or `Cancelled` to `New`
    ///
    /// The side effects for resetting an order are the same for both Expired and Cancelled orders.
    /// The effect is as if a new order comes in with the given details.
    ///
    /// * The order status is updated in the database.
    /// * The [`process_order`] flow is triggered.
    /// * An `OnOrderModified` event is triggered.
    /// * A `NewOrder` event is triggered.
    /// * An entry is added to the audit log.
    ///
    /// ### (3) Changing from `New` to `Paid`
    /// Usually, this change happens via the automated fulfillment flow. However, it can be done manually as well.
    /// When this happens, the following side effects occur:
    ///
    /// * A credit note for the `total_price` is created,
    /// * The `process_new_payment` flow is triggered, which will cause the order to be fulfilled and the status updated
    ///   to `Paid`.
    ///
    /// ### Changing from `Expired` to `Cancelled` or vice versa
    /// This change is forbidden and returns an error.
    ///
    /// ### Changing from `Paid` to `New`, `Expired`, or `Cancelled`
    /// These changes are forbidden and return an error.
    ///
    /// ### Changing from a status to itself.
    /// This change is a no-op and returns an error.
    ///
    /// ## Returns
    /// The old status of the order.
    pub async fn modify_status_for_order(
        &self,
        oid: &OrderId,
        new_status: OrderStatusType,
    ) -> Result<Order, PaymentGatewayError> {
        let order =
            self.db.fetch_order_by_order_id(oid).await?
                .ok_or_else(|| AccountApiError::dne(oid.clone()))?;
        let old_status = order.status;
        use crate::db_types::OrderStatusType::*;
        match (old_status, new_status) {
            (old, new) if old == new => return Err(PaymentGatewayError::OrderModificationNoOp),
            (New, Paid) => self.new_to_paid(order).await,
            (New, Expired | Cancelled) => self.cancel_or_expire_order(order, new_status).await,
            (Expired | Cancelled, New) => self.reset_order(order).await,
            (_, _) => return Err(PaymentGatewayError::OrderModificationForbidden),
        }
    }

    /// A manual order status transition from `New` to `Paid` status.
    /// This method is called by the default implementation of [`modify_status_for_order`] when the new status is
    /// `Paid`. When this happens, the following side effects occur:
    ///
    /// * A credit note for the `total_price` is created (TODO at API level),
    /// * The `process_new_payment` flow is triggered, which will cause the order to be fulfilled and the status updated
    ///   to `Paid`.(TODO at API level)
    async fn new_to_paid(&self, order: Order) -> Result<Order, PaymentGatewayError> {
        todo!()

    }

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
        order: Order,
        new_status: OrderStatusType,
    ) -> Result<Order, PaymentGatewayError> {
        todo!()
    }

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
    async fn reset_order(&self, order: Order) -> Result<Order, PaymentGatewayError> {
        todo!()
    }

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
    ) -> Result<(i64, i64), PaymentGatewayError> {
        todo!()
    }

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
    async fn modify_memo_for_order(&self, order_id: &OrderId, new_memo: &str) -> Result<Order, PaymentGatewayError> {
        todo!()
    }

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
    ) -> Result<Order, PaymentGatewayError> {
        todo!()
    }

    /// Since only XTR is supported currently, this method will always return an error.
    async fn modify_currency_for_order(
        &self,
        _order_id: &OrderId,
        _new_currency: &str,
    ) -> Result<Order, PaymentGatewayError> {
        Err(PaymentGatewayError::UnsupportedAction("Multiple currencies".to_string()))
    }


    pub fn db(&self) -> &B {
        &self.db
    }

    pub fn db_mut(&mut self) -> &mut B {
        &mut self.db
    }
}
