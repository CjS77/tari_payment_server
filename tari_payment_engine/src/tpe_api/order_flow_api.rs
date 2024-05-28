use std::fmt::Debug;

use log::*;

use crate::{
    db_types::{NewOrder, NewPayment, Order, TransferStatus},
    events::{EventProducers, OrderPaidEvent},
    traits::{PaymentGatewayDatabase, PaymentGatewayError},
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
    /// To change details about an order, you should use the [`update_order`] method.
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
    /// To change the status of a payment, you should use the [`update_payment_status`] method.
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

    /// Update an existing order in the order manager.
    pub async fn update_order(&self, _order: NewOrder) -> Result<i64, PaymentGatewayError> {
        todo!()
    }

    pub fn db(&self) -> &B {
        &self.db
    }

    pub fn db_mut(&mut self) -> &mut B {
        &mut self.db
    }
}
