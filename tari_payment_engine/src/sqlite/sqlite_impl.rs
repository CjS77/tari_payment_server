//! `SqliteDatabase` is a concrete implementation of a Tari Payment engine backend.
//!
//! Unsurprisingly, it uses SQLite as the backend and implements all the traits defined in the [`traits`] module.
use std::fmt::Debug;

use log::*;
use sqlx::SqlitePool;
use tari_common_types::tari_address::TariAddress;

use super::db::{auth, db_url, new_pool, orders, transfers, user_accounts, wallet_auth};
use crate::{
    db_types::{
        CreditNote,
        MicroTari,
        NewOrder,
        NewPayment,
        Order,
        OrderId,
        OrderStatusType,
        Payment,
        Role,
        TransferStatus,
        UserAccount,
    },
    order_objects::{ModifyOrderRequest, OrderQueryFilter},
    tpe_api::account_objects::FullAccount,
    traits::{
        AccountApiError,
        AccountManagement,
        AuthApiError,
        AuthManagement,
        NewWalletInfo,
        PaymentGatewayDatabase,
        PaymentGatewayError,
        UpdateWalletInfo,
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

    async fn fetch_or_create_account_for_order(&self, order: &NewOrder) -> Result<i64, PaymentGatewayError> {
        let mut conn = self.pool.acquire().await?;
        let cust_id = Some(order.customer_id.clone());
        let pubkey = order.address.as_ref().cloned();
        let id = user_accounts::fetch_or_create_account(cust_id, pubkey, &mut conn).await?;
        Ok(id)
    }

    async fn fetch_or_create_account_for_payment(&self, payment: &Payment) -> Result<i64, PaymentGatewayError> {
        let mut conn = self.pool.acquire().await?;
        let pubkey = Some(payment.sender.clone().to_address());
        let customer_id = match payment.order_id.as_ref() {
            Some(oid) => orders::fetch_order_by_order_id(oid, &mut conn).await?.map(|o| o.customer_id),
            None => None,
        };
        let id = user_accounts::fetch_or_create_account(customer_id, pubkey, &mut conn).await?;
        Ok(id)
    }

    async fn process_new_order_for_customer(&self, order: NewOrder) -> Result<i64, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let price = order.total_price;
        let id = orders::idempotent_insert(order.clone(), &mut tx).await?;
        debug!("🗃️ Order #{} has been saved in the DB with id {id}", order.order_id);
        let account_id =
            user_accounts::fetch_or_create_account(Some(order.customer_id.clone()), order.address, &mut tx).await?;
        user_accounts::incr_order_totals(account_id, price, price, &mut tx).await?;
        tx.commit().await?;
        Ok(account_id)
    }

    /// Takes a new payment, and in a single atomic transaction,
    /// * calls `save_payment` to store the payment in the database. If the payment already exists, nothing further is
    ///   done.
    /// * The payment is marked as `Unconfirmed`
    /// * creates a new account for the public key if one does not already exist
    /// * Adds the payment amount to the account's total received, and total pending
    /// Returns the account id for the public key.
    async fn process_new_payment_for_pubkey(&self, payment: NewPayment) -> Result<i64, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let txid = transfers::idempotent_insert(payment.clone(), &mut tx).await?;
        debug!("🗃️ Transfer {txid} received from [{}]", payment.sender.as_address());
        let customer_id = match &payment.order_id {
            Some(order_id) => {
                let existing_order = orders::fetch_order_by_order_id(order_id, &mut tx).await?;
                existing_order.map(|o| o.customer_id)
            },
            None => None,
        };
        let sender = Some(payment.sender.to_address());
        let acc_id = user_accounts::fetch_or_create_account(customer_id, sender, &mut tx).await?;
        user_accounts::adjust_balances(acc_id, payment.amount, payment.amount, MicroTari::from(0), &mut tx).await?;
        debug!("🗃️ Transfer {txid} processed. {} credited to pending account", payment.amount);
        tx.commit().await?;
        Ok(acc_id)
    }

    async fn process_credit_note_for_customer(&self, note: CreditNote) -> Result<i64, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let address = transfers::credit_note(note.clone(), &mut tx).await?;
        debug!("🗃️ Credit note for {} created with address {address}", note.customer_id);
        let CreditNote { amount, customer_id, .. } = note;
        let sender = Some(address.clone());
        let account_id = user_accounts::fetch_or_create_account(Some(customer_id), sender, &mut tx).await?;
        trace!("🗃️ Credit note: account {account_id} has been retrieved/created");
        let zero = MicroTari::from(0);
        user_accounts::adjust_balances(account_id, amount, zero, amount, &mut tx).await?;
        trace!("🗃️ Credit note: adjusting balances for account {account_id} by {amount}");
        tx.commit().await?;
        Ok(account_id)
    }

    async fn fetch_payable_orders(&self, account_id: i64) -> Result<Vec<Order>, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let account = user_accounts::user_account_by_id(account_id, &mut tx)
            .await?
            .ok_or(PaymentGatewayError::AccountNotFound(account_id))?;
        let query = OrderQueryFilter::default().with_account_id(account_id).with_status(OrderStatusType::New);
        let unpaid_orders = orders::search_orders(query, &mut tx).await?;
        let balance = account.current_balance;
        trace!("🗃️ Account #{account_id} has {} unpaid orders and a balance of {}.", unpaid_orders.len(), balance);
        let (paid_orders, _new_balance) =
            unpaid_orders.into_iter().fold((vec![], balance), |(mut orders, mut balance), order| {
                if balance >= order.total_price {
                    balance -= order.total_price;
                    orders.push(order);
                }
                (orders, balance)
            });
        tx.commit().await?;
        Ok(paid_orders)
    }

    async fn try_pay_orders(&self, account_id: i64, orders: &[Order]) -> Result<Vec<Order>, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let account = user_accounts::user_account_by_id(account_id, &mut tx)
            .await?
            .ok_or(PaymentGatewayError::AccountNotFound(account_id))?;
        let mut new_balance = account.current_balance;
        let mut result = Vec::with_capacity(orders.len());
        for order in orders {
            if new_balance >= order.total_price {
                new_balance -= order.total_price;
                orders::update_order_status(order.id, OrderStatusType::Paid, &mut tx).await?;
                trace!("🗃️ Order #{} of {} marked as paid", order.id, order.total_price);
                result.push(order.clone());
            }
        }
        let total_paid = account.current_balance - new_balance;
        if total_paid != MicroTari::from(0) {
            user_accounts::update_user_balance(account_id, new_balance, &mut tx).await?;
            trace!("Account {account_id} balance updated from {} to {new_balance}", account.current_balance);
            user_accounts::incr_order_totals(account_id, MicroTari::from(0), -total_paid, &mut tx).await?;
            trace!("🗃️ Adjusted account #{account_id} orders outstanding by {total_paid}.");
        }
        tx.commit().await?;
        Ok(result)
    }

    async fn update_payment_status(
        &self,
        txid: &str,
        status: TransferStatus,
    ) -> Result<Option<i64>, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let payment = transfers::fetch_payment(txid, &mut tx).await?;
        if payment.is_none() {
            return Err(PaymentGatewayError::PaymentStatusUpdateError(format!("Payment {txid} not found")));
        }
        let payment = payment.unwrap();
        let old_status = payment.status;
        trace!("🗃️ Updating payment: Payment {txid} is currently {old_status}");
        use TransferStatus::*;
        if old_status == status {
            debug!("🗃️ Payment {txid} already has status {status}. No action to take");
            return Ok(None);
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
        trace!("🗃️ Looking for account linked to payment {txid}");
        let account = match user_accounts::user_account_for_tx(txid, &mut tx).await {
            Ok(Some(acc)) => Ok(acc),
            Ok(None) => Err(PaymentGatewayError::AccountNotLinkedWithTransaction(format!(
                "No account is not linked to payment {txid}"
            ))),
            Err(e) => Err(e.into()),
        }?;
        let acc_id = account.id;
        let unchanged = MicroTari::from(0);
        let amount = payment.amount;
        transfers::update_status(txid, status, &mut tx).await?;

        match status {
            Confirmed => user_accounts::adjust_balances(acc_id, unchanged, -amount, amount, &mut tx).await?,
            Cancelled => user_accounts::adjust_balances(acc_id, -amount, -amount, unchanged, &mut tx).await?,
            _ => unreachable!(),
        };
        debug!("🗃️ Payment [{txid}] is now {status}. Balances have been updated.");
        tx.commit().await?;
        Ok(Some(acc_id))
    }

    /// A manual order status transition from `New` to `Paid` status.
    /// This method is called by the default implementation of [`modify_status_for_order`] when the new status is
    /// `Paid`. When this happens, the following side effects occur:
    /// * A credit note for the `total_price` is created,
    async fn new_to_paid(&self, order: Order) -> Result<Order, PaymentGatewayError> {
        if order.status != OrderStatusType::New {
            error!("🗃️ Order {} is not in 'New' status. Cannot call **new**_to_paid", order.id);
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        let mut conn = self.pool.acquire().await?;
        let update = ModifyOrderRequest::default().with_new_status(OrderStatusType::Paid);
        let order = orders::update_order(&order.order_id, update, &mut conn)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order.order_id.clone()))?;
        Ok(order)
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
    async fn cancel_or_expire_order(
        &self,
        order: Order,
        new_status: OrderStatusType,
    ) -> Result<Order, PaymentGatewayError> {
        if order.status != OrderStatusType::New {
            error!("🗃️ Order {} is not in 'New' status. Cannot call cancel_or_expire_order", order.id);
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        let update = ModifyOrderRequest::default().with_new_status(new_status);
        let mut conn = self.pool.begin().await?;
        let account = user_accounts::user_account_for_order(&order.order_id, &mut conn)
            .await?
            .ok_or_else(|| PaymentGatewayError::AccountShouldExistForOrder(order.order_id.clone()))?;
        let order = orders::update_order(&order.order_id, update, &mut conn)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order.order_id.clone()))?;
        user_accounts::incr_order_totals(account.id, -order.total_price, -order.total_price, &mut conn).await?;
        conn.commit().await?;
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
    /// * The [`process_order`] flow is triggered.
    async fn reset_order(&self, order: Order) -> Result<Order, PaymentGatewayError> {
        if !matches!(order.status, OrderStatusType::Expired | OrderStatusType::Cancelled) {
            error!("🗃️ Order {} is not in 'Expired' or 'Cancelled' status. Cannot call reset_order", order.id);
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        let update = ModifyOrderRequest::default().with_new_status(OrderStatusType::New);
        let mut tx = self.pool.begin().await?;
        let order = orders::update_order(&order.order_id, update, &mut tx)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order.order_id.clone()))?;

        let price = order.total_price;
        let account = user_accounts::user_account_for_customer_id(&order.customer_id, &mut tx)
            .await?
            .ok_or_else(|| PaymentGatewayError::AccountShouldExistForOrder(order.order_id.clone()))?;
        user_accounts::incr_order_totals(account.id, price, price, &mut tx).await?;
        tx.commit().await?;
        Ok(order)
    }

    /// Change the customer id for the given `order_id`. This function has several side effects:
    /// - The `customer_id` field of the order is updated in the database.
    /// - The total orders for the old and new customer are updated.
    /// - If the order is fulfillable with existing payments in the new account, the fulfillment flow is triggered.
    ///   (TODO at API level)
    /// - If the new customer does not exist, a new one is created.
    /// - If the order status was `Expired`, or `Cancelled`, it is **not** automatically reset to `New`. The admin must
    ///   follow up with a "change status" call to reset the order.
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
        new_cid: &str,
    ) -> Result<(i64, i64), PaymentGatewayError> {
        let update = ModifyOrderRequest::default().with_new_customer_id(new_cid);
        let mut conn = self.pool.begin().await?;
        let order = orders::fetch_order_by_order_id(order_id, &mut conn)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order_id.clone()))?;
        // Cannot change customer id on orders that have already been paid
        if matches!(order.status, OrderStatusType::Paid) {
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        let old_acc = user_accounts::user_account_for_order(order_id, &mut conn).await?;
        if old_acc.is_none() {
            warn!("Order {order_id} does not have an associated account. This should not happen.");
            conn.rollback().await?;
            return Err(PaymentGatewayError::AccountShouldExistForOrder(order_id.clone()));
        }
        let old_account = old_acc.unwrap();
        let order = orders::update_order(order_id, update, &mut conn).await?.ok_or_else(|| {
            warn!(
                "Order {order_id} does not exist, but we fetched in within this same transaction. This should not \
                 happen. There's a data race of sorts happening here and should be sorted out."
            );
            AccountApiError::OrderDoesNotExist(order_id.clone())
        })?;
        // Order is either expired, cancelled or new by now. If expired or cancelled, we don't need to make any
        // adjustments but new orders need to be accounted for.
        if let OrderStatusType::New = order.status {
            user_accounts::incr_order_totals(old_account.id, -order.total_price, -order.total_price, &mut conn).await?;
        }
        let cust_id = Some(new_cid.into());
        let new_account_id = user_accounts::fetch_or_create_account(cust_id, None, &mut conn).await?;
        if let OrderStatusType::New = order.status {
            let new_account = user_accounts::user_account_by_id(new_account_id, &mut conn).await?.ok_or_else(|| {
                warn!("Account {new_account_id} does not exist, but we just created it. This should not happen.");
                PaymentGatewayError::AccountShouldExistForOrder(order_id.clone())
            })?;
            let _ = user_accounts::incr_order_totals(new_account_id, order.total_price, order.total_price, &mut conn)
                .await?;
            orders::pay_order(&new_account, &order, &mut conn).await?;
        }
        conn.commit().await?;
        Ok((old_account.id, new_account_id))
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
        let order = orders::update_order(&order_id, update, &mut conn)
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
    ) -> Result<Order, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let order = orders::fetch_order_by_order_id(order_id, &mut tx)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order_id.clone()))?;
        if !matches!(order.status, OrderStatusType::New) {
            info!("🗃️ Order {order_id}'s price cannot be changed since it is already {}", order.status);
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        let update = ModifyOrderRequest::default().with_new_total_price(new_total_price);
        let order = orders::update_order(order_id, update, &mut tx).await?.ok_or_else(|| {
            let msg = format!(
                "Order {order_id} does not exist, but we fetched in within this same transaction. This represents a \
                 bug and the transaction will be rolled back"
            );
            error!("{msg}");
            PaymentGatewayError::DatabaseError(msg)
        })?;
        Ok(order)
    }

    async fn close(&mut self) -> Result<(), PaymentGatewayError> {
        self.pool.close().await;
        Ok(())
    }
}

impl AccountManagement for SqliteDatabase {
    async fn fetch_user_account(&self, account_id: i64) -> Result<Option<UserAccount>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        user_accounts::user_account_by_id(account_id, &mut conn).await
    }

    /// Fetches the user account for the given order id. A user account must have already been created for this account.
    /// If no account is found, `None` will be returned.
    async fn fetch_user_account_for_order(&self, order_id: &OrderId) -> Result<Option<UserAccount>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        user_accounts::user_account_for_order(order_id, &mut conn).await
    }

    async fn fetch_user_account_for_customer_id(
        &self,
        customer_id: &str,
    ) -> Result<Option<UserAccount>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        user_accounts::user_account_for_customer_id(customer_id, &mut conn).await
    }

    async fn fetch_user_account_for_address(
        &self,
        pubkey: &TariAddress,
    ) -> Result<Option<UserAccount>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        user_accounts::user_account_for_address(pubkey, &mut conn).await
    }

    async fn fetch_orders_for_account(&self, account_id: i64) -> Result<Vec<Order>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let query = OrderQueryFilter::default().with_account_id(account_id);
        let orders = orders::search_orders(query, &mut conn).await?;
        Ok(orders)
    }

    async fn fetch_order_by_order_id(&self, order_id: &OrderId) -> Result<Option<Order>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let oid = orders::fetch_order_by_order_id(order_id, &mut conn).await?;
        Ok(oid)
    }

    async fn fetch_payments_for_address(&self, address: &TariAddress) -> Result<Vec<Payment>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let payments = transfers::fetch_payments_for_address(address, &mut conn).await?;
        Ok(payments)
    }

    async fn history_for_address(&self, address: &TariAddress) -> Result<Option<FullAccount>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let id = match user_accounts::user_account_for_address(address, &mut conn).await? {
            Some(acc) => acc.id,
            None => return Ok(None),
        };
        user_accounts::history_for_id(id, &mut conn).await
    }

    async fn history_for_id(&self, id: i64) -> Result<Option<FullAccount>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        user_accounts::history_for_id(id, &mut conn).await
    }

    async fn search_orders(
        &self,
        mut query: OrderQueryFilter,
        only_for: Option<TariAddress>,
    ) -> Result<Vec<Order>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        if let Some(address) = only_for {
            let id = match user_accounts::user_account_for_address(&address, &mut conn).await? {
                Some(acc) => acc.id,
                None => return Ok(vec![]),
            };
            query = query.with_account_id(id);
        }
        let orders = orders::search_orders(query, &mut conn).await?;
        Ok(orders)
    }

    async fn creditors(&self) -> Result<Vec<UserAccount>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let accounts = user_accounts::creditors(&mut conn).await?;
        Ok(accounts)
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
        debug!("🔑️ Roles {roles:?} assigned to {}", address.to_hex());
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

    async fn deregister_wallet(&self, _wallet_address: &TariAddress) -> Result<WalletInfo, WalletManagementError> {
        todo!()
    }

    async fn update_wallet_info(&self, _wallet: UpdateWalletInfo) -> Result<(), WalletManagementError> {
        todo!()
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
}
