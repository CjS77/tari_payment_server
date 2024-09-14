//! `SqliteDatabase` is a concrete implementation of a Tari Payment engine backend.
//!
//! Unsurprisingly, it uses SQLite as the backend and implements all the traits defined in the [`traits`] module.
use std::{cmp::Reverse, fmt::Debug};

use chrono::Duration;
use log::*;
use sqlx::SqlitePool;
use tari_common_types::tari_address::TariAddress;
use tpg_common::MicroTari;

use super::db::{accounts, auth, db_url, exchange_rates, new_pool, orders, transfers, wallet_auth};
use crate::{
    db_types::{
        AddressBalance,
        CreditNote,
        CustomerBalance,
        CustomerOrderBalance,
        CustomerOrders,
        NewOrder,
        NewPayment,
        NewSettlementJournalEntry,
        Order,
        OrderId,
        OrderStatusType,
        Payment,
        Role,
        SerializedTariAddress,
        SettlementType,
        TransferStatus,
    },
    order_objects::{ModifyOrderRequest, OrderChanged, OrderQueryFilter},
    tpe_api::{
        account_objects::{AddressHistory, CustomerHistory, Pagination},
        exchange_objects::ExchangeRate,
    },
    traits::{
        AccountApiError,
        AccountManagement,
        AuthApiError,
        AuthManagement,
        ExchangeRateError,
        ExchangeRates,
        ExpiryResult,
        MultiAccountPayment,
        NewWalletInfo,
        OrderMovedResult,
        PaymentGatewayDatabase,
        PaymentGatewayError,
        WalletAuth,
        WalletAuthApiError,
        WalletInfo,
        WalletManagement,
        WalletManagementError,
    },
};

#[derive(Clone)]
pub struct SqliteDatabase {
    url: String,
    pool: SqlitePool,
}

impl Debug for SqliteDatabase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SqliteDatabase ({:?})", self.pool)
    }
}

impl PaymentGatewayDatabase for SqliteDatabase {
    fn url(&self) -> &str {
        self.url.as_str()
    }

    async fn claim_order(&self, order_id: &OrderId, address: &TariAddress) -> Result<Order, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let order = orders::fetch_order_by_order_id(order_id, &mut tx)
            .await?
            .ok_or_else(|| PaymentGatewayError::OrderNotFound(order_id.clone()))?;
        let addr58 = address.to_base58();
        if order.status != OrderStatusType::Unclaimed {
            warn!(
                "🖇️️ Order {} is not 'Unclaimed' and {addr58} is trying to claim it. The current status is {}",
                order.order_id, order.status
            );
        }
        let order = orders::update_order_status(order.id, OrderStatusType::New, &mut tx).await?;
        accounts::link_address_to_customer(address, &order.customer_id, &mut tx).await?;
        info!("🗃️ Address {addr58} has been linked with customer id {}", order.customer_id);
        tx.commit().await?;
        Ok(order)
    }

    async fn auto_claim_order(&self, order: &Order) -> Result<Option<(TariAddress, Order)>, PaymentGatewayError> {
        if order.status != OrderStatusType::Unclaimed {
            error!("🖇️️ Order {} is not 'Unclaimed' and cannot be auto-claimed", order.order_id);
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        let mut tx = self.pool.begin().await?;
        let cust_id = &order.customer_id;
        let address = accounts::balances_for_customer_id(cust_id, &mut tx).await?;
        // The first address is the most recent one
        let Some(address) = address.first().map(|a| a.address().clone()) else {
            // We could omit the tx commit here, 'cos we're not making any changes
            tx.commit().await?;
            return Ok(None);
        };
        let order = orders::update_order_status(order.id, OrderStatusType::New, &mut tx).await?;
        tx.commit().await?;
        Ok(Some((address, order)))
    }

    async fn insert_order(&self, order: NewOrder) -> Result<(Order, bool), PaymentGatewayError> {
        let mut conn = self.pool.acquire().await?;
        let result = orders::idempotent_insert(order, &mut conn).await?;
        Ok(result)
    }

    /// Takes a new payment, and in a single atomic transaction,
    /// * calls `save_payment` to store the payment in the database. If the payment already exists, nothing further is
    ///   done.
    /// * The payment is marked as `Unconfirmed`
    /// * Adds the payment amount to the account's total received, and total pending
    ///
    /// Returns the newly created Payment record.
    async fn process_new_payment(&self, payment: NewPayment) -> Result<Payment, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let maybe_order_id = payment.order_id.clone();
        debug!("🗃️ Payment {} received from [{}]", payment.txid, payment.sender.as_address());
        let payment = transfers::idempotent_insert(payment, &mut tx).await?;
        // If the order id is already known, link the address and customer_id
        if let Some(order_id) = maybe_order_id {
            accounts::link_address_to_order(&order_id, payment.sender.as_address(), &mut tx).await?;
            info!("🗃️ Address {} linked to order {order_id}", payment.sender.as_address());
        }
        debug!("🗃️ Transfer {} processed. {} credited to pending account", payment.txid, payment.amount);
        tx.commit().await?;
        Ok(payment)
    }

    async fn fetch_pending_payments_for_address(
        &self,
        address: &TariAddress,
    ) -> Result<Vec<Payment>, PaymentGatewayError> {
        let mut conn = self.pool.acquire().await?;
        let payments = transfers::pending_payments(address, &mut conn).await?;
        Ok(payments)
    }

    async fn process_credit_note_for_customer(&self, note: CreditNote) -> Result<Payment, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let payment = transfers::credit_note(&note, &mut tx).await?;
        debug!("🗃️ Credit note for {} created with address {}", note.customer_id, payment.sender.as_address());
        let address = payment.sender.as_address();
        accounts::link_address_to_customer(address, &note.customer_id, &mut tx).await?;
        debug!("🗃️ Dummy wallet {} linked to customer id {}", address.to_base58(), note.customer_id);
        tx.commit().await?;
        Ok(payment)
    }

    async fn fetch_payable_orders_for_address(&self, address: &TariAddress) -> Result<Vec<Order>, PaymentGatewayError> {
        let mut conn = self.pool.acquire().await?;
        let orders = orders::fetch_payable_orders_for_address(address, &mut conn).await?;
        Ok(orders)
    }

    /// Tries to pay for a single order from any wallet associated with the order's customer Id.
    ///
    /// It's possible to pay for the order from multiple wallets, in which case the settlement type will be `Multiple`,
    /// with the sum total of the payments being equal to the order's total price.
    async fn try_pay_order(&self, order: &Order) -> Result<Option<MultiAccountPayment>, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let mut balances = accounts::balances_for_customer_id(&order.customer_id, &mut tx).await?;
        let mut total_due = order.total_price;
        let total_credit = balances.iter().map(|b| b.current_balance()).sum();
        if balances.is_empty() || (total_due > total_credit) {
            let err = PaymentGatewayError::AccountError(AccountApiError::InsufficientFunds);
            return Err(err);
        }
        // Sort the balances in descending order of current balance
        balances.sort_by_key(|b| Reverse(b.current_balance()));
        // Preferably, use a `Single` journal entry type
        let settlement_type =
            if balances[0].current_balance() >= total_due { SettlementType::Single } else { SettlementType::Multiple };
        let mut result = MultiAccountPayment::new(vec![], vec![]);
        let zero = MicroTari::from(0);
        for account in balances {
            let address = SerializedTariAddress::from(account.address());
            let amount_paid = account.current_balance().min(total_due);
            total_due -= amount_paid;
            let settlement = NewSettlementJournalEntry {
                order_id: order.order_id.clone(),
                payment_address: address,
                amount: amount_paid,
                settlement_type,
            };
            let settlement = accounts::insert_settlement(settlement, &mut tx).await?;
            result.settlements.push(settlement);
            if total_due == zero {
                break;
            }
        }
        if total_due == zero {
            let paid_order = orders::update_order_status(order.id, OrderStatusType::Paid, &mut tx).await?;
            result.orders_paid.push(paid_order);
        }
        tx.commit().await?;
        Ok(if result.orders_paid.is_empty() { None } else { Some(result) })
    }

    /// Tries to fulfil the orders using the address as payment source.
    ///
    /// This method will not try and use other addresses that are also linked to the customer ids in the order list.
    async fn try_pay_orders_from_address(
        &self,
        address: &TariAddress,
        orders: &[&Order],
    ) -> Result<Option<MultiAccountPayment>, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let result = self.pay_orders_for_address_with_conn(address, orders, &mut tx).await?;
        tx.commit().await?;
        Ok(result)
    }

    async fn update_payment_status(&self, txid: &str, status: TransferStatus) -> Result<Payment, PaymentGatewayError> {
        let mut conn = self.pool.acquire().await?;
        let Some(payment) = transfers::fetch_payment(txid, &mut conn).await? else {
            return Err(PaymentGatewayError::PaymentStatusUpdateError(format!("Payment {txid} not found")));
        };
        let old_status = payment.status;
        trace!("🗃️ Updating payment: Payment {txid} is currently {old_status}");
        use TransferStatus::*;
        if old_status == status {
            debug!("🗃️ Payment {txid} already has status {status}. No action to take");
            return Err(PaymentGatewayError::PaymentModificationNoOp);
        }
        if old_status != Received {
            error!(
                "🗃️ Payment {txid} cannot be transitioned from {old_status} to {status}.If there is a valid use case, \
                 perform a manual adjustment now and submit a ticket so that it can be handled properly in the future."
            );
            return Err(PaymentGatewayError::PaymentStatusUpdateError(format!(
                "Payment {txid} has status {status} instead of 'Received'"
            )));
        }

        let payment = transfers::update_status(txid, status, &mut conn).await?;
        debug!("🗃️ Payment [{txid}] is now {status}.");
        Ok(payment)
    }

    async fn fetch_payment_by_tx_id(&self, tx_id: &str) -> Result<Payment, PaymentGatewayError> {
        let mut conn = self.pool.acquire().await?;
        let payment = transfers::fetch_payment(tx_id, &mut conn).await?;
        payment.ok_or_else(|| PaymentGatewayError::PaymentNotFound(tx_id.into()))
    }

    /// A manual order status transition from `New` to `Paid` status.
    /// A credit note for the `total_price` is created.
    async fn mark_new_order_as_paid(&self, order: Order, reason: &str) -> Result<Order, PaymentGatewayError> {
        if order.status != OrderStatusType::New {
            error!("🗃️ Order {} is not in 'New' status. Cannot call **new**_to_paid", order.id);
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        let mut tx = self.pool.begin().await?;
        let reason = format!("Admin credit overrode for order {}. Reason: {reason}", order.order_id);
        let note = CreditNote::new(order.customer_id.clone(), order.total_price).with_reason(reason);
        let payment = transfers::credit_note(&note, &mut tx).await?;
        let address = payment.sender.to_address();
        debug!(
            "🗃️ Credit note: Customer {} received note for {} with address {}",
            order.customer_id,
            order.total_price,
            address.to_base58(),
        );
        let result = self.pay_orders_for_address_with_conn(&address, &[&order], &mut tx).await?;
        if result.is_none() {
            error!(
                "🗃️ Order {} could not be paid for atfer issuing a credit note for the full amount. This is most \
                 likely a bug",
                order.id
            );
            return Err(PaymentGatewayError::OrderNotFound(order.order_id));
        }
        tx.commit().await?;
        let updated_order = result.unwrap().orders_paid.remove(0);
        Ok(updated_order)
    }

    /// A manual order status transition from `New` to `Expired` or `Cancelled` status.
    ///
    /// The side effects for expiring or cancelling an order are the same. The only difference is that Expired orders
    /// are triggered automatically based on time, whereas cancelling an order is triggered by an admin or a shopify
    /// webhook.
    ///
    /// * The order status is updated in the database.
    /// * The total orders for the account are updated.
    async fn cancel_or_expire_order(
        &self,
        order_id: &OrderId,
        new_status: OrderStatusType,
        reason: &str,
    ) -> Result<Order, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let order = orders::fetch_order_by_order_id(order_id, &mut tx)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order_id.clone()))?;
        if !&[OrderStatusType::New, OrderStatusType::Unclaimed].contains(&order.status) {
            error!("🗃️ Order {} is not in 'New' status. Cannot call cancel_or_expire_order", order.id);
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        let update = ModifyOrderRequest::default().with_new_status(new_status).with_new_memo(reason);
        let order = orders::update_order(&order.order_id, update, &mut tx)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order.order_id.clone()))?;
        tx.commit().await?;
        Ok(order)
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
    async fn reset_order(&self, order_id: &OrderId) -> Result<OrderChanged, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let old_order = orders::fetch_order_by_order_id(order_id, &mut tx)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order_id.clone()))?;
        if !matches!(old_order.status, OrderStatusType::Expired | OrderStatusType::Cancelled) {
            error!("🗃️ Order {} is not in 'Expired' or 'Cancelled' status. Cannot call reset_order", old_order.id);
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        let update = ModifyOrderRequest::default().with_new_status(OrderStatusType::New);
        let updated_order = orders::update_order(&old_order.order_id, update, &mut tx)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(old_order.order_id.clone()))?;
        tx.commit().await?;
        let result = OrderChanged::new(old_order, updated_order);
        Ok(result)
    }

    /// Change the customer id for the given `order_id`. This function has several side effects:
    /// - The `customer_id` field of the order is updated in the database.
    /// - If the new customer does not exist, a new one is created.
    /// - If the order status was `Expired`, or `Cancelled`, it is **not** automatically reset to `New`. The admin must
    ///   follow up with a "change status" call to reset the order.
    ///
    /// ## Returns:
    /// - The updated order, if it was paid for by the new account.
    ///
    /// ## Failure modes:
    /// - If the order does not exist, an error is returned.
    /// - If the order status is already `Paid`, an error is returned.
    async fn modify_customer_id_for_order(
        &self,
        order_id: &OrderId,
        new_cid: &str,
    ) -> Result<OrderMovedResult, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let old_order = orders::fetch_order_by_order_id(order_id, &mut tx)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order_id.clone()))?;
        // Cannot change customer id on orders that have already been paid
        if matches!(old_order.status, OrderStatusType::Paid) {
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        if new_cid == old_order.customer_id {
            debug!("🗃️ Order {order_id} is being reassigned to the same customer. No action taken.");
            tx.rollback().await?;
            return Err(PaymentGatewayError::OrderModificationNoOp);
        }
        let update = ModifyOrderRequest::default().with_new_customer_id(new_cid);
        let mut new_order = orders::update_order(order_id, update, &mut tx).await?.ok_or_else(|| {
            error!(
                "Order {order_id} does not exist, but we fetched it within this same transaction. This should not \
                 happen. There's a data race of sorts happening here and should be sorted out."
            );
            AccountApiError::OrderDoesNotExist(order_id.clone())
        })?;
        // Order is either expired, cancelled or new by now. If expired or cancelled, we don't need to make any
        // adjustments but new orders need to be accounted for.
        tx.commit().await?;

        let mut settlements = Vec::new();
        if let OrderStatusType::New = new_order.status {
            match self.try_pay_order(&new_order).await {
                Ok(Some(payment)) => {
                    let mut orders_paid;
                    MultiAccountPayment { settlements, orders_paid, .. } = payment;
                    orders_paid.drain(..1).for_each(|o| new_order = o);
                },
                Ok(None) => { /* noop */ },
                Err(PaymentGatewayError::AccountError(AccountApiError::InsufficientFunds)) => {
                    debug!(
                        "🗃️ There weren't enough funds to pay for order {order_id} from the new customer id {} \
                         immediately",
                        new_cid
                    );
                },
                Err(e) => return Err(e),
            };
        }
        let result = OrderMovedResult::new(old_order, new_order, settlements);
        Ok(result)
    }

    /// Changes the memo field for an order.
    /// Changing the memo does not trigger any other flows, does not affect
    /// the order status, and does not affect order fulfillment.
    ///
    /// ## Returns:
    /// The modified order
    async fn modify_memo_for_order(&self, order_id: &OrderId, new_memo: &str) -> Result<Order, PaymentGatewayError> {
        let update = ModifyOrderRequest::default().with_new_memo(new_memo);
        let mut conn = self.pool.acquire().await?;
        let order = orders::update_order(order_id, update, &mut conn)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order_id.clone()))?;
        Ok(order)
    }

    /// Changes the total price for an order.
    ///
    /// To return successfully, the order must exist, and have `New` status.
    /// This function has several side effects:
    /// - The `total_price` field of the order is updated in the database.
    /// - The total orders for the account are updated.
    ///
    /// ## Failure modes:
    /// - If the order does not exist.
    /// - If the order status was `Expired`, or `Cancelled` or `Paid`.
    async fn modify_total_price_for_order(
        &self,
        order_id: &OrderId,
        new_total_price: MicroTari,
    ) -> Result<OrderChanged, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let old_order = orders::fetch_order_by_order_id(order_id, &mut tx)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order_id.clone()))?;
        if !matches!(old_order.status, OrderStatusType::New) {
            info!("🗃️ Order {order_id}'s price cannot be changed since it is already {}", old_order.status);
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        if old_order.total_price == new_total_price {
            info!("🗃️ Order {order_id}'s price is already {new_total_price}. No action taken.");
            return Err(PaymentGatewayError::OrderModificationNoOp);
        }
        let update = ModifyOrderRequest::default().with_new_total_price(new_total_price);
        let new_order = orders::update_order(order_id, update, &mut tx).await?.ok_or_else(|| {
            let msg = format!(
                "Order {order_id} does not exist, but we fetched in within this same transaction. This represents a \
                 bug and the transaction will be rolled back"
            );
            error!("{msg}");
            PaymentGatewayError::DatabaseError(msg)
        })?;
        tx.commit().await?;
        let delta = OrderChanged::new(old_order, new_order);
        Ok(delta)
    }

    async fn expire_old_orders(
        &self,
        unclaimed_limit: Duration,
        unpaid_limit: Duration,
    ) -> Result<ExpiryResult, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let unclaimed_orders = orders::expire_orders(OrderStatusType::Unclaimed, unclaimed_limit, &mut tx).await?;
        let unpaid_orders = orders::expire_orders(OrderStatusType::New, unpaid_limit, &mut tx).await?;
        tx.commit().await?;
        Ok(ExpiryResult::new(unclaimed_orders, unpaid_orders))
    }

    async fn close(&mut self) -> Result<(), PaymentGatewayError> {
        self.pool.close().await;
        Ok(())
    }
}

impl AccountManagement for SqliteDatabase {
    async fn fetch_orders_for_address(&self, address: &TariAddress) -> Result<Vec<Order>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let orders = accounts::orders_for_address(address, &mut conn).await?;
        Ok(orders)
    }

    async fn fetch_order_by_order_id(&self, order_id: &OrderId) -> Result<Option<Order>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let order = orders::fetch_order_by_order_id(order_id, &mut conn).await?;
        Ok(order)
    }

    async fn fetch_payments_for_address(&self, address: &TariAddress) -> Result<Vec<Payment>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let payments = transfers::fetch_payments_for_address(address, &mut conn).await?;
        Ok(payments)
    }

    async fn history_for_address(&self, address: &TariAddress) -> Result<AddressHistory, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let balance = accounts::fetch_address_balance(address, &mut conn).await?;
        let payments = transfers::fetch_payments_for_address(address, &mut conn).await?;
        let orders = accounts::orders_for_address(address, &mut conn).await?;
        let settlements = accounts::settlements_for_address(address, &mut conn).await?;
        let address = SerializedTariAddress::from(address.clone());
        let history = AddressHistory::new(address, balance, orders, payments, settlements);
        Ok(history)
    }

    async fn history_for_customer(&self, customer_id: &str) -> Result<CustomerHistory, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let balances = accounts::balances_for_customer_id(customer_id, &mut conn).await?;
        let balance = CustomerBalance::new(balances);
        let order_balance = accounts::customer_order_balance(customer_id, &mut conn).await?;
        let query = OrderQueryFilter::default().with_customer_id(customer_id.to_string());
        let orders = orders::search_orders(query, &mut conn).await?;
        let settlements = accounts::settlements_for_customer_id(customer_id, &mut conn).await?;
        let history = CustomerHistory::builder(customer_id.to_string())
            .balance(balance)
            .order_balance(order_balance)
            .orders(orders)
            .settlements(settlements)
            .build()?;
        Ok(history)
    }

    async fn search_orders(&self, query: OrderQueryFilter) -> Result<Vec<Order>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let orders = orders::search_orders(query, &mut conn).await?;
        Ok(orders)
    }

    async fn creditors(&self) -> Result<Vec<CustomerOrders>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let accounts = accounts::creditors(&mut conn).await?;
        Ok(accounts)
    }

    async fn fetch_customer_ids(&self, pagination: &Pagination) -> Result<Vec<String>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let ids = accounts::customer_ids(pagination, &mut conn).await?;
        Ok(ids)
    }

    async fn fetch_addresses(&self, pagination: &Pagination) -> Result<Vec<TariAddress>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let addresses = accounts::addresses(pagination, &mut conn).await?;
        Ok(addresses)
    }

    async fn fetch_address_balance(&self, address: &TariAddress) -> Result<AddressBalance, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let account = accounts::fetch_address_balance(address, &mut conn).await?;
        Ok(account)
    }

    async fn fetch_customer_balance(&self, customer_id: &str) -> Result<CustomerBalance, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let balances = accounts::balances_for_customer_id(customer_id, &mut conn).await?;
        let balance = CustomerBalance::new(balances);
        Ok(balance)
    }

    async fn fetch_customer_order_balance(&self, customer_id: &str) -> Result<CustomerOrderBalance, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let balances = accounts::customer_order_balance(customer_id, &mut conn).await?;
        Ok(balances)
    }
}

impl AuthManagement for SqliteDatabase {
    async fn check_auth_account_exists(&self, address: &TariAddress) -> Result<bool, AuthApiError> {
        let mut conn = self.pool.acquire().await.map_err(|e| AuthApiError::DatabaseError(e.to_string()))?;
        auth::auth_account_exists(address, &mut conn).await
    }

    async fn check_address_has_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<(), AuthApiError> {
        let mut conn = self.pool.acquire().await.map_err(|e| AuthApiError::DatabaseError(e.to_string()))?;
        auth::address_has_roles(address, roles, &mut conn).await
    }

    async fn fetch_roles_for_address(&self, address: &TariAddress) -> Result<Vec<Role>, AuthApiError> {
        let mut conn = self.pool.acquire().await.map_err(|e| AuthApiError::DatabaseError(e.to_string()))?;
        let roles = auth::roles_for_address(address, &mut conn).await?;
        Ok(roles.into_iter().collect())
    }

    async fn create_auth_log(&self, _address: &TariAddress, _nonce: u64) -> Result<(), AuthApiError> {
        // Sqlite uses upsert
        Ok(())
    }

    // Overriding this because we can use upserts
    async fn upsert_nonce_for_address(&self, address: &TariAddress, nonce: u64) -> Result<(), AuthApiError> {
        self.update_nonce_for_address(address, nonce).await
    }

    // This implementation is an upsert under the hood
    async fn update_nonce_for_address(&self, address: &TariAddress, nonce: u64) -> Result<(), AuthApiError> {
        let mut conn = self.pool.acquire().await.map_err(|e| AuthApiError::DatabaseError(e.to_string()))?;
        auth::upsert_nonce_for_address(address, nonce, &mut conn).await
    }

    async fn assign_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<(), AuthApiError> {
        let mut tx = self.pool.begin().await.map_err(|e| AuthApiError::DatabaseError(e.to_string()))?;
        auth::assign_roles(address, roles, &mut tx).await?;
        tx.commit().await.map_err(|e| AuthApiError::DatabaseError(e.to_string()))?;
        debug!("🔑️ Roles {roles:?} assigned to {}", address.to_base58());
        Ok(())
    }

    async fn remove_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<u64, AuthApiError> {
        let mut conn = self.pool.acquire().await.map_err(|e| AuthApiError::DatabaseError(e.to_string()))?;
        auth::remove_roles(address, roles, &mut conn).await
    }
}

impl WalletAuth for SqliteDatabase {
    async fn get_wallet_info(&self, wallet_address: &TariAddress) -> Result<WalletInfo, WalletAuthApiError> {
        let mut conn = self.pool.acquire().await?;
        let result = wallet_auth::fetch_wallet_info_for_address(wallet_address, &mut conn).await?;
        Ok(result)
    }

    async fn update_wallet_nonce(
        &self,
        wallet_address: &TariAddress,
        new_nonce: i64,
    ) -> Result<(), WalletAuthApiError> {
        let mut conn = self.pool.acquire().await?;
        wallet_auth::update_wallet_nonce(wallet_address, new_nonce, &mut conn).await?;
        Ok(())
    }
}

impl WalletManagement for SqliteDatabase {
    async fn register_wallet(&self, wallet: NewWalletInfo) -> Result<(), WalletManagementError> {
        let mut conn = self.pool.acquire().await?;
        wallet_auth::register_wallet(wallet, &mut conn).await
    }

    async fn deregister_wallet(&self, _wallet_address: &TariAddress) -> Result<(), WalletManagementError> {
        let mut conn = self.pool.acquire().await?;
        wallet_auth::deregister_wallet(_wallet_address, &mut conn).await
    }

    async fn fetch_authorized_wallets(&self) -> Result<Vec<WalletInfo>, WalletManagementError> {
        let mut conn = self.pool.acquire().await?;
        wallet_auth::fetch_authorized_wallets(&mut conn).await
    }
}

impl ExchangeRates for SqliteDatabase {
    async fn fetch_last_rate(&self, currency: &str) -> Result<ExchangeRate, ExchangeRateError> {
        let mut conn = self.pool.acquire().await.map_err(|e| ExchangeRateError::DatabaseError(e.to_string()))?;
        exchange_rates::fetch_last_rate(currency, &mut conn).await
    }

    /// Save the exchange rate for the given currency to the backend storage
    ///
    /// The `updated_at` field of the exchange rate is ignored. The backend will set this field to the current time.
    async fn set_exchange_rate(&self, new_rate: &ExchangeRate) -> Result<(), ExchangeRateError> {
        let mut conn = self.pool.acquire().await.map_err(|e| ExchangeRateError::DatabaseError(e.to_string()))?;
        exchange_rates::set_exchange_rate(new_rate, &mut conn).await
    }
}

impl SqliteDatabase {
    /// Creates a new database API object
    pub async fn new(max_connections: u32) -> Result<Self, sqlx::Error> {
        let url = db_url();
        SqliteDatabase::new_with_url(url.as_str(), max_connections).await
    }

    pub async fn new_with_url(url: &str, max_connections: u32) -> Result<Self, sqlx::Error> {
        trace!("Creating new database connection pool with url {url}");
        let pool = new_pool(url, max_connections).await?;
        let url = url.to_string();
        Ok(Self { url, pool })
    }

    /// Returns a reference to the database connection pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    async fn pay_orders_for_address_with_conn(
        &self,
        address: &TariAddress,
        orders: &[&Order],
        tx: &mut sqlx::SqliteConnection,
    ) -> Result<Option<MultiAccountPayment>, PaymentGatewayError> {
        let balance = accounts::fetch_address_balance(address, tx).await?;
        let mut remaining_credit = balance.current_balance();
        trace!("🗃️ Address balance of {} is {remaining_credit}", address.to_base58());
        let mut paid_orders = Vec::with_capacity(orders.len());
        let mut settlements = Vec::with_capacity(orders.len());
        for &order in orders {
            // We must be able to pay for the entire order, or no deal.
            trace!("🗃️ Checking if there's enough credit ({remaining_credit}) to pay for order [{}]", order.order_id);
            if order.total_price > remaining_credit {
                break;
            }
            trace!("🗃️ Order [{}] can be paid", order.order_id);
            remaining_credit -= order.total_price;
            let settlement = NewSettlementJournalEntry {
                order_id: order.order_id.clone(),
                payment_address: SerializedTariAddress::from(address.clone()),
                amount: order.total_price,
                settlement_type: SettlementType::Single,
            };
            let settlement = accounts::insert_settlement(settlement, tx).await?;
            trace!("🗃️ Settlement journal entry created for order [{}] (id: {})", order.order_id, order.id);
            settlements.push(settlement);
            let updated_order = orders::update_order_status(order.id, OrderStatusType::Paid, tx).await?;
            debug!("🗃️ Order {} paid for during multi-account payment", order.id);
            paid_orders.push(updated_order);
        }

        let result = (!paid_orders.is_empty()).then(|| MultiAccountPayment::new(paid_orders, settlements));
        Ok(result)
    }
}
