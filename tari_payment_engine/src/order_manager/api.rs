use crate::db_types::{NewOrder, NewPayment, Order, TransferStatus};
use crate::order_manager::OrderManagerError;
use crate::{InsertOrderResult, PaymentGatewayDatabase};
use log::*;
use std::fmt::Debug;

pub struct OrderManagerApi<B> {
    db: B,
}

impl<B> Debug for OrderManagerApi<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OrderManagerApi")
    }
}

impl<B> OrderManagerApi<B> {
    pub fn new(db: B) -> Self {
        Self { db }
    }
}

impl<B> OrderManagerApi<B>
where
    B: PaymentGatewayDatabase,
{
    /// Submit a new order to the order manager.
    ///
    /// This should be a brand-new order. If the order already exists, the order manager will return an error.
    /// To change details about an order, you should use the [`update_order`] method.
    ///
    /// After the order is added, all the orders for the account are checked to see if any can be marked as paid.
    /// If any orders are marked as paid, they are returned.
    pub async fn process_new_order(
        &self,
        order: NewOrder,
    ) -> Result<Vec<Order>, OrderManagerError<B>> {
        let account_id = self
            .db
            .process_new_order_for_customer(order.clone())
            .await
            .map_err(|e| OrderManagerError::DatabaseError(e))?;
        let paid_orders = self
            .db
            .fetch_payable_orders(account_id)
            .await
            .map_err(|e| OrderManagerError::DatabaseError(e))?;
        debug!(
            "🔄️ {} orders marked as paid for account #{account_id}",
            paid_orders.len()
        );
        Ok(paid_orders)
    }

    /// Submit a new payment to the order manager.
    ///
    /// This should be a brand-new payment. If the payment already exists, the order manager will return an error.
    /// To change the status of a payment, you should use the [`update_payment_status`] method.
    ///
    /// After the payment is added, all the orders for the account are checked to see if any can be marked as paid.
    /// If any orders are marked as paid, they are returned.
    pub async fn process_new_payment(
        &self,
        payment: NewPayment,
    ) -> Result<Vec<Order>, OrderManagerError<B>> {
        let txid = payment.txid.clone();
        let account_id = self
            .db
            .process_new_payment_for_pubkey(payment.clone())
            .await
            .map_err(|e| OrderManagerError::DatabaseError(e))?;
        trace!("🔄️💰️ Payment [{txid}] for account #{account_id} processed.");
        let payable = self
            .db
            .fetch_payable_orders(account_id)
            .await
            .map_err(|e| OrderManagerError::DatabaseError(e))?;
        trace!(
            "🔄️💰️ {} fulfillable orders fetched for account #{account_id}",
            payable.len()
        );
        let paid_orders = self
            .db
            .try_pay_orders(account_id, &payable)
            .await
            .map_err(|e| OrderManagerError::DatabaseError(e))?;
        debug!("🔄️💰️ Payment [{txid}] processing complete. {} orders are paid for account #{account_id}",payable.len());
        Ok(paid_orders)
    }

    pub async fn confirm_transaction(
        &self,
        txid: String,
    ) -> Result<Vec<Order>, OrderManagerError<B>> {
        trace!("🔄️✅️ Payment {txid} is being marked as confirmed");
        let account_id = self
            .db
            .update_payment_status(&txid, TransferStatus::Confirmed)
            .await
            .map_err(|e| OrderManagerError::DatabaseError(e))?;
        let paid_orders = match account_id {
            Some(acc_id) => {
                let payable = self
                    .db
                    .fetch_payable_orders(acc_id)
                    .await
                    .map_err(|e| OrderManagerError::DatabaseError(e))?;
                trace!(
                    "🔄️✅️ {} fulfillable orders fetched for account #{acc_id}",
                    payable.len()
                );
                let paid_orders = self
                    .db
                    .try_pay_orders(acc_id, &payable)
                    .await
                    .map_err(|e| OrderManagerError::DatabaseError(e))?;
                debug!(
                    "🔄️✅️ [{txid}] confirmed. {} orders are paid for account #{acc_id}",
                    payable.len()
                );
                paid_orders
            }
            None => {
                error!("🔄️✅️ [{txid}] confirmed, but it is not linked to any account!");
                Vec::new()
            }
        };
        Ok(paid_orders)
    }

    /// Update an existing order in the order manager.
    pub async fn update_order(
        &self,
        _order: NewOrder,
    ) -> Result<InsertOrderResult, OrderManagerError<B>> {
        todo!()
    }

    pub fn db(&self) -> &B {
        &self.db
    }

    pub fn db_mut(&mut self) -> &mut B {
        &mut self.db
    }
}
