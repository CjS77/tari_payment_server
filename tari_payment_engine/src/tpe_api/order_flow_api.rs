use std::fmt::Debug;

use chrono::Duration;
use log::*;
use tari_common_types::tari_address::TariAddress;
use tpg_common::MicroTari;

use crate::{
    db_types::{CreditNote, NewOrder, NewPayment, Order, OrderId, OrderStatusType, Payment, TransferStatus},
    events::{EventProducers, OrderAnnulledEvent, OrderClaimedEvent, OrderEvent, OrderModifiedEvent, PaymentEvent},
    helpers::MemoSignature,
    order_objects::{ClaimedOrder, OrderChanged},
    traits::{
        AccountApiError,
        ExpiryResult,
        MultiAccountPayment,
        OrderMovedResult,
        PaymentGatewayDatabase,
        PaymentGatewayError,
    },
};

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
    ///
    /// If `auto_claim` is set to `true`, we will look for any existing wallets that are associated with the customer
    /// id on the order and immediately claim the order.
    pub async fn process_new_order(&self, order: NewOrder, auto_claim: bool) -> Result<Order, PaymentGatewayError> {
        let address = order.address.clone();
        let (mut order, inserted) = self.db.insert_order(order.clone()).await?;
        let id = order.order_id.clone();
        if !inserted {
            info!("🔄️📦️ Order [{id}] has already been processed.");
            return Err(PaymentGatewayError::OrderAlreadyExists(id.clone()));
        }
        if let Some(address) = &address {
            debug!("🔄️📦️ Order [{id}] has an address attached. Claiming immediately.");
            order = self.db.claim_order(&order.order_id, address).await?;
            info!("🔄️📦️ Order [{id}] has been claimed immediately by address {}", address.to_base58());
        }
        self.call_new_order_hook(&order).await;
        // If the address is already known, we can try pay for the order immediately
        if let Some(address) = address {
            debug!("🔄️📦️ Order [{id}] has an address attached. Trying to pay immediately.");
            if let Some(result) = self.try_pay_orders_from_address(&address, &[&order]).await? {
                let maybe_paid = result.orders_paid.into_iter().find(|o| o.order_id == order.order_id);
                if let Some(paid_order) = maybe_paid {
                    info!("🔄️📦️ Order [{id}] has been settled immediately by the address attached to the order.");
                    order = paid_order
                }
            }
        }
        if auto_claim && order.status == OrderStatusType::Unclaimed {
            if let Some((address, updated_order)) = self.db.auto_claim_order(&order).await? {
                info!("🔄️📦️ Order [{id}] has been auto-claimed by address {}", address.to_base58());
                order = updated_order;
                if let Some(mut payments) = self.try_pay_orders_from_address(&address, &[&order]).await? {
                    let addresses = payments
                        .settlements
                        .iter()
                        .map(|s| s.payment_address.as_base58())
                        .collect::<Vec<_>>()
                        .join(", ");
                    info!(
                        "🔄️📦️ Order [{id}] has been settled immediately as a result of auto-claiming. Payment \
                         addresses were {addresses}"
                    );
                    order = payments.orders_paid.remove(0);
                }
            } else {
                debug!("🔄️📦️ Order [{id}] could not be auto-claimed. No addresses are linked to the customer yet.");
            }
        }
        info!("🔄️📦️ Order [{id}] processing complete.");
        Ok(order)
    }

    /// Claims an order for a Tari wallet address.
    ///
    /// This function:
    /// * Checks that the signature is valid,
    /// * Checks that the order exists, and is in the `Unclaimed` status,
    ///
    /// If these checks pass, then
    /// * The wallet address is linked with the customer ID
    /// * the order is `Claimed`
    /// * The OrderClaimed event is fired.
    pub async fn claim_order(&self, signature: &MemoSignature) -> Result<ClaimedOrder, PaymentGatewayError> {
        if !signature.is_valid() {
            return Err(PaymentGatewayError::InvalidSignature);
        }
        let order_id = OrderId(signature.order_id.clone());
        let address = signature.address.as_address();
        debug!("🖇️️ Claiming order [{order_id}] for address {}", address.to_base58());
        let mut order = self.db.claim_order(&order_id, address).await?;
        self.call_order_claimed_hook(&order, address).await;
        // If payment is successful, order OrderPaid trigger will fire implicitly.
        trace!("🖇️📦️ Checking if order [{}] can be fulfilled immediately", order.order_id);
        if let Some(mut result) = self.try_pay_orders_from_address(address, &[&order]).await? {
            order = result.orders_paid.remove(0);
        } else {
            trace!("🖇️📦️ Order [{}] cannot be fulfilled immediately", order.order_id);
        }
        let claimed_order = ClaimedOrder::from(order);
        Ok(claimed_order)
    }

    async fn call_order_paid_hook(&self, paid_orders: &[Order]) {
        if paid_orders.is_empty() {
            return;
        }
        debug!(
            "🔄️📦️ Notifying {} OrderPaid hook subscribers about {} paid orders",
            self.producers.order_paid_producer.len(),
            paid_orders.len()
        );
        for emitter in &self.producers.order_paid_producer {
            for order in paid_orders {
                let event = OrderEvent { order: order.clone() };
                emitter.publish_event(event).await;
            }
        }
    }

    async fn call_new_order_hook(&self, new_order: &Order) {
        for emitter in &self.producers.new_order_producer {
            debug!("🔄️📦️ Notifying new order hook subscribers");
            let event = OrderEvent { order: new_order.clone() };
            emitter.publish_event(event).await;
        }
    }

    /// Calls the registered function when an order is cancelled or expired
    async fn call_order_annulled_hook(&self, updated_order: &Order) {
        debug!("🔄️📦️ Notifying order annulled hook subscribers");
        for emitter in &self.producers.order_annulled_producer {
            let event = OrderAnnulledEvent::new(updated_order.clone());
            emitter.publish_event(event).await;
        }
    }

    /// Calls the registered function when an order is claimed by a wallet address
    async fn call_order_claimed_hook(&self, order: &Order, address: &TariAddress) {
        debug!("🔄️📦️ Notifying {} order claimed hook subscribers", self.producers.order_claimed_producer.len());
        let event = OrderClaimedEvent::new(order.clone(), address.clone());
        for emitter in &self.producers.order_claimed_producer {
            emitter.publish_event(event.clone()).await;
        }
    }

    async fn call_order_modified_hook(&self, field: &str, orders: OrderChanged) {
        debug!("🔄️📦️ Notifying order modified hook subscribers");
        let event = OrderModifiedEvent::new(field.to_string(), orders);
        for emitter in &self.producers.order_modified_producer {
            emitter.publish_event(event.clone()).await;
        }
    }

    async fn call_payment_received_hook(&self, payment: &Payment) {
        debug!("🔄️💰️ Notifying payment received hook subscribers");
        for emitter in &self.producers.payment_received_producer {
            let event = PaymentEvent::new(payment.clone());
            emitter.publish_event(event).await;
        }
    }

    async fn call_payment_confirmed_hook(&self, payment: &Payment) {
        debug!("🔄️💰️ Notifying payment confirmed hook subscribers");
        for emitter in &self.producers.payment_confirmed_producer {
            let event = PaymentEvent::new(payment.clone());
            emitter.publish_event(event).await;
        }
    }

    /// Submit a new payment to the order manager.
    ///
    /// This should be a brand-new payment. If the payment already exists, the order manager will return an error.
    /// To change the status of a payment, you should use the [`Self::confirm_payment`] or [`Self::cancel_payment`]
    /// methods.
    ///
    /// New payments are always unconfirmed, so no orders can be paid at this point, so no checking of orders is done.
    ///
    /// The newly inserted payment record is returned.
    pub async fn process_new_payment(&self, payment: NewPayment) -> Result<Payment, PaymentGatewayError> {
        let txid = payment.txid.clone();
        let payment = self.db.process_new_payment(payment.clone()).await?;
        info!("🔄️💰️ Payment [{txid}] for {} processed.", payment.sender.as_base58());
        self.call_payment_received_hook(&payment).await;
        Ok(payment)
    }

    pub async fn issue_credit_note(
        &self,
        note: CreditNote,
    ) -> Result<Option<MultiAccountPayment>, PaymentGatewayError> {
        let cust_id = note.customer_id.clone();
        debug!("🔄️💰️ Issuing credit note for customer {cust_id}");
        let payment = self.db.process_credit_note_for_customer(note).await?;
        info!("🔄️💰️ Credit note issued for customer {cust_id}");
        self.call_payment_received_hook(&payment).await;
        // A credit note is a confirmed payment so do the same things we would do for a confirmed payment
        let payment = self.post_confirm(&payment, false).await?;
        Ok(payment)
    }

    /// Update the status of a payment to "Confirmed". This happens when a transaction in the blockchain is deep enough
    /// in the chain that a re-org and invalidation of the payment is unlikely.
    pub async fn confirm_payment(&self, txid: String) -> Result<Payment, PaymentGatewayError> {
        trace!("🔄️✅️ Payment {txid} is being marked as confirmed");
        let payment = self.db.update_payment_status(&txid, TransferStatus::Confirmed).await?;
        // Setting isolated mode to true here, but we might want this to be false? Need feedback from users.
        let paid_orders = self.post_confirm(&payment, true).await?;
        let n = paid_orders.map(|p| p.orders_paid.len()).unwrap_or(0);
        info!("🔄️✅️ Payment {txid} was confirmed. {n} orders were paid as a result.");
        Ok(payment)
    }

    /// Try and pay for orders after a confirmation. If `isolated` is true, then _only_ funds in the confirmed payment
    /// address are used to pay for orders. If `isolated` is false, then all orders for the customer are considered.
    async fn post_confirm(
        &self,
        payment: &Payment,
        isolated: bool,
    ) -> Result<Option<MultiAccountPayment>, PaymentGatewayError> {
        debug!("🔄️✅️ Executing post-payment confirmation actions for {}", payment.txid);
        let address = payment.sender.as_address();
        let payable = self.db.fetch_payable_orders_for_address(address).await?;
        let orders = payable.iter().collect::<Vec<&Order>>();
        trace!("🔄️✅️ {} fulfillable orders fetched for {address}", payable.len());
        let result = if isolated {
            info!("🔄️✅️ Paying for orders in isolation mode");
            self.try_pay_orders_from_address(address, &orders).await?
        } else {
            info!("🔄️✅️ Paying for orders using any address associated with the orders");
            self.try_pay_orders(&orders).await?
        };
        let txid = &payment.txid;
        let mut log_msg = format!("[{txid}] confirmed.");
        if let Some(orders_paid) = &result {
            log_msg += &format!(" {} orders are paid for address {address}", orders_paid.orders_paid.len());
        }
        debug!("🔄️✅️ {log_msg}");
        self.call_payment_confirmed_hook(payment).await;
        Ok(result)
    }

    /// Mark a payment as cancelled and update orders and accounts as necessary.
    pub async fn cancel_payment(&self, txid: String) -> Result<(), PaymentGatewayError> {
        trace!("🔄️❌️ Payment {txid} is being marked as cancelled");
        self.db.update_payment_status(&txid, TransferStatus::Cancelled).await?;
        info!("🔄️❌️ Payment {txid} was cancelled");
        Ok(())
    }

    /// A manual order status transition from `New` to `Paid` status.
    /// This method is called by the default implementation of [`modify_status_for_order`] when the new status is
    /// `Paid`. When this happens, the following side effects occur:
    ///
    /// * A credit note for the `total_price` is created,
    /// * The engine tries to pay for the order. Barring an odd data race, this should always succeed.
    /// * The order paid trigger is called.
    pub async fn mark_new_order_as_paid(&self, order_id: &OrderId, reason: &str) -> Result<Order, PaymentGatewayError> {
        let order = self
            .db
            .fetch_order_by_order_id(order_id)
            .await?
            .ok_or_else(|| PaymentGatewayError::OrderNotFound(order_id.clone()))?;
        // We don't call self.issue_credit_note() here because we want to force this specific order to get paid
        // The former lets any valid order be paid once the credit is issued.
        let updated_order = self.db.mark_new_order_as_paid(order, reason).await?;
        if updated_order.status == OrderStatusType::Paid {
            self.call_order_paid_hook(&[updated_order.clone()]).await;
            info!(
                "🔄️💲️ Order [{order_id}] was manually marked as paid and customer was credited with the full order \
                 amount."
            );
        } else {
            error!("🔄️💲️ Order [{order_id}] was not marked as paid. This almost certainly a bug.");
        }
        Ok(updated_order)
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
    /// * The [`OrderAnnulledEvent`] event is triggered.
    /// * An audit log entry is made.
    pub async fn cancel_or_expire_order(
        &self,
        order_id: &OrderId,
        new_status: OrderStatusType,
        reason: &str,
    ) -> Result<Order, PaymentGatewayError> {
        let updated_order = self.db.cancel_or_expire_order(order_id, new_status, reason).await?;
        self.call_order_annulled_hook(&updated_order).await;
        Ok(updated_order)
    }

    /// Manually reset an order from `Expired` or `Cancelled` status to `New` status.
    ///
    /// The side effects for resetting an order are the same for both Expired and Cancelled orders.
    /// The effect is as if a new order comes in with the given details.
    ///
    /// The reset causes the following side effects:
    /// * Resets the order status to `New`.
    /// * Calls the `OrderModified` event trigger.
    /// * Calls the `NewOrder` event trigger.
    /// * Tries to pay for the order, and if successful, triggers the `OrderPaid` event.
    /// * The audit log gets a new entry.
    pub async fn reset_order(&self, order_id: &OrderId) -> Result<OrderChanged, PaymentGatewayError> {
        debug!("🔄️📦️ Resetting order [{}]", order_id);
        let mut changes = self.db.reset_order(order_id).await?;
        self.call_order_modified_hook("status", changes.clone()).await;
        self.call_new_order_hook(&changes.new_order).await;
        if let Some(mut paid_order) = self.try_pay_order(&changes.new_order).await? {
            changes.new_order = paid_order.orders_paid.remove(0);
        }
        Ok(changes)
    }

    /// Tries to pay for an order using any addresses associated with the customer id attached to this order.
    /// If you've claimed an order, or otherwise know which address you want to pay from, use
    /// [`try_pay_orders_from_address`] instead.
    pub async fn try_pay_order(&self, order: &Order) -> Result<Option<MultiAccountPayment>, PaymentGatewayError> {
        match self.db.try_pay_order(order).await {
            Ok(None) => Ok(None),
            Ok(Some(result)) => {
                if result.orders_paid.is_empty() {
                    error!("🔄️📦️ If try_pay_order returns `Some`, there should be at least one order paid.");
                }
                self.call_order_paid_hook(&result.orders_paid).await;
                Ok(Some(result))
            },
            Err(PaymentGatewayError::AccountError(AccountApiError::InsufficientFunds)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Tries to pay for the given set of orders using _any_ funds associated with the customer id attached to the
    /// order. If you want to pay for orders using a specific address, use [`try_pay_orders_from_address`] instead.
    /// This function will try to pay for each order in the list, and return a single payment object that contains all
    /// the orders that were paid for.
    ///
    /// If no orders were paid, the function returns `Ok(None)`.
    pub async fn try_pay_orders(&self, orders: &[&Order]) -> Result<Option<MultiAccountPayment>, PaymentGatewayError> {
        let mut payments = Vec::new();
        for order in orders {
            if let Some(o) = self.try_pay_order(order).await? {
                payments.push(o);
            }
        }
        Ok(MultiAccountPayment::merge(payments))
    }

    /// Tries to pay for the given set of orders using _only_ funds from the given address.
    /// See [`PaymentGatewayDatabase::try_pay_orders_from_address`] for more details.
    pub async fn try_pay_orders_from_address(
        &self,
        address: &TariAddress,
        orders: &[&Order],
    ) -> Result<Option<MultiAccountPayment>, PaymentGatewayError> {
        let result = self.db.try_pay_orders_from_address(address, orders).await?;
        if let Some(payments) = &result {
            self.call_order_paid_hook(&payments.orders_paid).await;
        }
        Ok(result)
    }

    /// Change the customer id for the given `order_id`. This function has several side effects:
    /// - The `customer_id` field of the order is updated in the database.
    /// - The total orders for the old and new customer are updated.
    /// - If the order is fulfillable with existing payments in the new account, the fulfillment flow is triggered.
    /// - If the new customer does not exist, a new one is created.
    /// - If the order status was `Expired`, or `Cancelled`, it is **not** automatically reset to `New`. The admin must
    ///   follow up with a "change status" call to reset the order.
    /// - The `OnOrderModified` event is triggered.
    /// - If the order was filled, an `OnOrderPaid` event is triggered.
    ///
    /// ## Returns:
    /// - A [`OrderMovedResult`] object, which contains the old and new account ids, the orders that were moved, and
    ///   whether the order was fulfilled.
    ///
    /// ## Failure modes:
    /// - If the order does not exist, the method returns an error.
    /// - If the order status is already `Paid`, the method returns an error.
    pub async fn assign_order_to_new_customer(
        &self,
        order_id: &OrderId,
        new_cust_id: &str,
    ) -> Result<OrderMovedResult, PaymentGatewayError> {
        let move_result = self.db.modify_customer_id_for_order(order_id, new_cust_id).await?;
        self.call_order_modified_hook("customer_id", move_result.orders.clone()).await;
        if let Some(order) = move_result.filled_order() {
            self.call_order_paid_hook(&[order]).await;
        }
        Ok(move_result)
    }

    /// Changes the memo field for an order.
    ///
    /// This function has the following side effects.
    /// - The `OnOrderModified` event is triggered.
    ///
    /// Changing the memo does not trigger any other flows, does not affect
    /// the order status, and does not affect order fulfillment.
    ///
    /// ## Returns:
    /// The modified order
    pub async fn update_memo_for_order(
        &self,
        order_id: &OrderId,
        new_memo: &str,
    ) -> Result<Order, PaymentGatewayError> {
        debug!("🔄️📦️ Changing memo for order [{}]", order_id);
        let old_order = self
            .db
            .fetch_order_by_order_id(order_id)
            .await?
            .ok_or_else(|| PaymentGatewayError::OrderNotFound(order_id.clone()))?;
        let new_order = self.db.modify_memo_for_order(order_id, new_memo).await?;
        let changes = OrderChanged::new(old_order, new_order.clone());
        info!(
            "🔄️📦️ Memo for order [{}] changed from '{}' to '{}'",
            order_id,
            changes.old_order.memo.clone().unwrap_or_default(),
            new_memo
        );
        self.call_order_modified_hook("memo", changes).await;
        Ok(new_order)
    }

    /// Changes the total price for an order.
    ///
    /// To return successfully, the order must exist, and have `New` status.
    /// This function has several side effects:
    /// - The `total_price` field of the order is updated in the database.
    /// - The total orders for the account are updated.
    /// - If the order is now fulfillable with existing payments in the account, the fulfillment flow is triggered
    /// - An entry in the audit log is made.
    /// - The `OnOrderModified` event is triggered.
    ///
    /// ## Failure modes:
    /// - If the order does not exist.
    /// - If the order status was `Expired`, or `Cancelled`.
    /// - If the order status is `Paid`. To handle refunds or post-payment adjustments, use the `credit_note` function.
    ///
    /// ## Returns
    /// The modified order
    pub async fn update_price_for_order(
        &self,
        order_id: &OrderId,
        new_price: MicroTari,
    ) -> Result<Order, PaymentGatewayError> {
        if new_price < MicroTari::from(0) {
            warn!("🔄️💲️ An attempt was made to set order [{order_id}] to a negative value ({new_price})");
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        debug!("🔄️💲️ Changing price for order [{}]", order_id);
        let OrderChanged { old_order, mut new_order } =
            self.db.modify_total_price_for_order(order_id, new_price).await?;
        let direction = if old_order.total_price > new_order.total_price { "DECREASED" } else { "INCREASED" };
        info!(
            "🔄️💲️ Price for order [{order_id}] {direction} from {} to {}",
            old_order.total_price, new_order.total_price
        );
        if let Some(payments) = self.try_pay_order(&new_order).await? {
            // can panic, but try_pay_order should return None if there are no paid orders
            let paid_order = payments.to_order();
            debug!("🔄️💲️ Price change has led to order {} being Paid", paid_order.order_id);
            self.call_order_paid_hook(&[paid_order.clone()]).await;
            new_order = paid_order;
        }
        let changes = OrderChanged::new(old_order, new_order.clone());
        self.call_order_modified_hook("total_price", changes).await;
        Ok(new_order)
    }

    /// Since only XTR is supported currently, this method will always return an error.
    pub async fn modify_currency_for_order(
        &self,
        _order_id: &OrderId,
        _new_currency: &str,
    ) -> Result<Order, PaymentGatewayError> {
        Err(PaymentGatewayError::UnsupportedAction("Multiple currencies".to_string()))
    }

    pub async fn expire_old_orders(
        &self,
        unclaimed_expiry: Duration,
        unpaid_expiry: Duration,
    ) -> Result<ExpiryResult, PaymentGatewayError> {
        let result = self.db.expire_old_orders(unclaimed_expiry, unpaid_expiry).await?;
        for order in &result.unclaimed {
            self.call_order_annulled_hook(order).await;
        }
        for order in &result.unpaid {
            self.call_order_annulled_hook(order).await;
        }
        Ok(result)
    }

    pub fn db(&self) -> &B {
        &self.db
    }

    pub fn db_mut(&mut self) -> &mut B {
        &mut self.db
    }
}
