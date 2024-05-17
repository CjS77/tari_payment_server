use std::fmt::Debug;

use futures_util::future::LocalBoxFuture;
use log::*;

use crate::{
    db_types::{NewOrder, NewPayment, Order, TransferStatus},
    tpe_api::OrderManagerError,
    InsertOrderResult,
    PaymentGatewayDatabase,
};

pub type OrderCreatedHookFn = Box<dyn Fn(NewOrder) -> LocalBoxFuture<'static, ()> + Sync>;
pub type PaymentCreatedHookFn = Box<dyn Fn(NewPayment) -> LocalBoxFuture<'static, ()> + Sync>;

/// `OrderFlowApi` is the primary API for handling order and payment flows in response to merchant order events and
/// wallet payment events.
pub struct OrderFlowApi<B> {
    db: B,
    on_order_created: Option<OrderCreatedHookFn>,
    on_payment_created: Option<PaymentCreatedHookFn>,
}

impl<B> Debug for OrderFlowApi<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OrderManagerApi")
    }
}

impl<B> OrderFlowApi<B> {
    pub fn new(db: B) -> Self {
        Self { db, on_order_created: None, on_payment_created: None }
    }

    /// Add a hook that is called when a new order is created.
    ///
    /// The hook is called with the new order as an argument.
    /// Example:
    /// ```rust,ignore
    ///    let mut api = OrderManagerApi::new(db);
    ///    api.add_order_created_hook(Box::new(move |order| {
    ///      let fut = Box::pin( async { /* async code codes here */ });
    ///      fut.boxed_local()
    /// }));
    pub fn add_order_created_hook(&mut self, hook: OrderCreatedHookFn) {
        self.on_order_created = Some(hook);
    }

    /// Add a hook that is called when a new payment is created.
    /// See [`add_order_created_hook`] for an example.
    pub fn add_payment_created_hook(&mut self, hook: PaymentCreatedHookFn) {
        self.on_payment_created = Some(hook);
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
    pub async fn process_new_order(&self, order: NewOrder) -> Result<Vec<Order>, OrderManagerError<B>> {
        let account_id = self
            .db
            .process_new_order_for_customer(order.clone())
            .await
            .map_err(|e| OrderManagerError::DatabaseError(e))?;
        if let Some(hook) = &self.on_order_created {
            trace!("🔄️📦️ Executing OnOrderCreated hook for [{}].", order.order_id);
            hook(order.clone()).await;
        }
        let payable =
            self.db.fetch_payable_orders(account_id).await.map_err(|e| OrderManagerError::DatabaseError(e))?;
        let paid_orders =
            self.db.try_pay_orders(account_id, &payable).await.map_err(|e| OrderManagerError::DatabaseError(e))?;
        debug!(
            "🔄️📦️ Order [{}] processing complete. {} orders are paid for account #{account_id}",
            order.order_id,
            payable.len()
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
    pub async fn process_new_payment(&self, payment: NewPayment) -> Result<Vec<Order>, OrderManagerError<B>> {
        let txid = payment.txid.clone();
        let account_id = self
            .db
            .process_new_payment_for_pubkey(payment.clone())
            .await
            .map_err(|e| OrderManagerError::DatabaseError(e))?;
        trace!("🔄️💰️ Payment [{txid}] for account #{account_id} processed.");
        if let Some(hook) = &self.on_payment_created {
            trace!("🔄️💰️ Executing OnPayment hook for [{txid}].");
            hook(payment.clone()).await;
        }
        let payable =
            self.db.fetch_payable_orders(account_id).await.map_err(|e| OrderManagerError::DatabaseError(e))?;
        trace!("🔄️💰️ {} fulfillable orders fetched for account #{account_id}", payable.len());
        let paid_orders =
            self.db.try_pay_orders(account_id, &payable).await.map_err(|e| OrderManagerError::DatabaseError(e))?;
        debug!(
            "🔄️💰️ Payment [{txid}] processing complete. {} orders are paid for account #{account_id}",
            payable.len()
        );
        Ok(paid_orders)
    }

    /// Update the status of a payment to "Confirmed". This happens when a transaction in the blockchain is deep enough
    /// in the chain that a re-org and invalidation of the payment is unlikely.
    pub async fn confirm_payment(&self, txid: String) -> Result<Vec<Order>, OrderManagerError<B>> {
        trace!("🔄️✅️ Payment {txid} is being marked as confirmed");
        let account_id = self
            .db
            .update_payment_status(&txid, TransferStatus::Confirmed)
            .await
            .map_err(|e| OrderManagerError::DatabaseError(e))?;
        let paid_orders = match account_id {
            Some(acc_id) => {
                let payable =
                    self.db.fetch_payable_orders(acc_id).await.map_err(|e| OrderManagerError::DatabaseError(e))?;
                trace!("🔄️✅️ {} fulfillable orders fetched for account #{acc_id}", payable.len());
                let paid_orders =
                    self.db.try_pay_orders(acc_id, &payable).await.map_err(|e| OrderManagerError::DatabaseError(e))?;
                debug!("🔄️✅️ [{txid}] confirmed. {} orders are paid for account #{acc_id}", payable.len());
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
    pub async fn cancel_payment(&self, txid: String) -> Result<(), OrderManagerError<B>> {
        trace!("🔄️❌️ Payment {txid} is being marked as cancelled");
        self.db
            .update_payment_status(&txid, TransferStatus::Cancelled)
            .await
            .map_err(|e| OrderManagerError::DatabaseError(e))?;
        Ok(())
    }

    /// Update an existing order in the order manager.
    pub async fn update_order(&self, _order: NewOrder) -> Result<InsertOrderResult, OrderManagerError<B>> {
        todo!()
    }

    pub fn db(&self) -> &B {
        &self.db
    }

    pub fn db_mut(&mut self) -> &mut B {
        &mut self.db
    }
}
