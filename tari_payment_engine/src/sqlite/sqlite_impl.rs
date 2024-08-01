//! `SqliteDatabase` is a concrete implementation of a Tari Payment engine backend.
//!
//! Unsurprisingly, it uses SQLite as the backend and implements all the traits defined in the [`traits`] module.
use std::fmt::Debug;

use chrono::Duration;
use log::*;
use sqlx::SqlitePool;
use tari_common_types::tari_address::TariAddress;
use tpg_common::MicroTari;

use super::db::{auth, db_url, exchange_rates, new_pool, orders, transfers, user_accounts, wallet_auth};
use crate::{
    db_types::{
        CreditNote,
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
    order_objects::{ModifyOrderRequest, OrderChanged, OrderQueryFilter},
    tpe_api::{
        account_objects::{FullAccount, Pagination},
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
        let is_claimed = order.address.is_some();
        let account_id =
            user_accounts::fetch_or_create_account(Some(order.customer_id.clone()), order.address, &mut tx).await?;
        if is_claimed {
            orders::update_order_status(id, OrderStatusType::New, &mut tx).await?;
        }
        user_accounts::incr_order_totals(account_id, price, price, &mut tx).await?;
        tx.commit().await?;
        Ok(account_id)
    }

    /// Takes a new payment, and in a single atomic transaction,
    /// * calls `save_payment` to store the payment in the database. If the payment already exists, nothing further is
    ///   done.
    /// * The payment is marked as `Unconfirmed`
    /// * creates a new account for the address if one does not already exist
    /// * Adds the payment amount to the account's total received, and total pending
    ///
    /// Returns the account id for the public key.
    async fn process_new_payment_for_pubkey(&self, payment: NewPayment) -> Result<(i64, Payment), PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let payment = transfers::idempotent_insert(payment.clone(), &mut tx).await?;
        debug!("🗃️ Payment {} received from [{}]", payment.txid, payment.sender.as_address());
        let customer_id = match &payment.order_id {
            Some(order_id) => {
                let existing_order = orders::fetch_order_by_order_id(order_id, &mut tx).await?;
                existing_order.map(|o| o.customer_id)
            },
            None => None,
        };
        let sender = Some(payment.sender.as_address().clone());
        let acc_id = match user_accounts::fetch_or_create_account(customer_id, sender, &mut tx).await {
            Ok(id) => id,
            Err(AccountApiError::AmbiguousAccounts(info)) => {
                // There are multiple accounts for the same address-customer pair. So we need to disambiguate
                info!(
                    "🗃️ The payment from address {addr} specified an order to pay, but it is already associated with \
                     customer {cust} on account {o_acc}.I'm going to create a new entry linking the address {addr} to \
                     account {o_acc}",
                    addr = info.address,
                    cust = info.customer_id,
                    o_acc = info.customer_account_id
                );
                user_accounts::link_accounts(info.customer_account_id, None, Some(info.address), &mut tx).await?
            },
            Err(e) => return Err(e.into()),
        };
        user_accounts::adjust_balances(acc_id, payment.amount, payment.amount, MicroTari::from(0), &mut tx).await?;
        debug!("🗃️ Transfer {} processed. {} credited to pending account", payment.txid, payment.amount);
        tx.commit().await?;
        Ok((acc_id, payment))
    }

    async fn process_credit_note_for_customer(&self, note: CreditNote) -> Result<(i64, Payment), PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let payment = transfers::credit_note(note.clone(), &mut tx).await?;
        debug!("🗃️ Credit note for {} created with address {}", note.customer_id, payment.sender.as_address());
        let CreditNote { amount, customer_id, .. } = note;
        let sender = Some(payment.sender.as_address().clone());
        let account_id = user_accounts::fetch_or_create_account(Some(customer_id), sender, &mut tx).await?;
        trace!("🗃️ Credit note: account {account_id} has been retrieved/created");
        let zero = MicroTari::from(0);
        user_accounts::adjust_balances(account_id, amount, zero, amount, &mut tx).await?;
        trace!("🗃️ Credit note: adjusting balances for account {account_id} by {amount}");
        tx.commit().await?;
        Ok((account_id, payment))
    }

    async fn fetch_payable_orders(&self, account_id: i64) -> Result<Vec<Order>, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let account = user_accounts::user_account_by_id(account_id, &mut tx)
            .await?
            .ok_or(PaymentGatewayError::AccountNotFound(account_id))?;
        let query = OrderQueryFilter::default()
            .with_account_id(account_id)
            .with_status(OrderStatusType::Unclaimed)
            .with_status(OrderStatusType::New);
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
                let updated_order = orders::update_order_status(order.id, OrderStatusType::Paid, &mut tx).await?;
                trace!("🗃️ Order #{} of {} marked as paid", order.id, order.total_price);
                result.push(updated_order);
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

    async fn try_pay_orders_from_address(
        &self,
        address: &TariAddress,
        orders: &[&Order],
    ) -> Result<MultiAccountPayment, PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        // Fetch accounts with oldest account LAST
        let mut accounts = user_accounts::fetch_accounts_for_address(address, false, &mut tx).await?;
        let mut total_credit = accounts.iter().map(|acc| acc.current_balance).sum::<MicroTari>();
        trace!("🗃️ Address {address} has {} accounts with a total credit of {total_credit}", accounts.len());
        let mut paid_orders = Vec::with_capacity(orders.len());
        let mut used_accounts: Vec<(i64, MicroTari)> = Vec::with_capacity(accounts.len());
        for &order in orders {
            // We must be able to pay for the entire order, or no deal.
            trace!("🗃️ Trying to pay order {}", order.id);
            if order.total_price > total_credit {
                break;
            }
            trace!("🗃️ Order {} can be paid", order.id);
            let mut outstanding_amount = order.total_price;
            // Pay for order using oldest accounts FIRST (they're at the end of the list)
            while accounts[0].current_balance < outstanding_amount {
                let Some(used_acc) = accounts.pop() else {
                    error!(
                        "🗃️ Not enough funds to partially pay for order {} from address {address}. This is a bug, \
                         because I've just checked that there are enough funds to cover the payment. Aborting this \
                         transaction, and fix this ASAP",
                        order.id
                    );
                    return Err(PaymentGatewayError::DatabaseError("Internal bug. Check Logs".into()));
                };
                if used_acc.current_balance == MicroTari::from(0) {
                    trace!("🗃️ Skipping account {} because it has a balance of 0", used_acc.id);
                    continue;
                }
                trace!(
                    "🗃️ Used {} in account {} to pay part of order {}",
                    used_acc.current_balance,
                    used_acc.id,
                    order.id
                );
                total_credit -= used_acc.current_balance;
                outstanding_amount -= used_acc.current_balance;
                used_accounts.push((used_acc.id, used_acc.current_balance));
            }
            // This top account now has enough balance to pay for the order
            let Some(used_acc) = accounts.last_mut() else {
                error!(
                    "🗃️ Not enough funds to complete payment for order {} from address {address}. This is a bug, \
                     because I've just checked that there are enough funds to cover the payment. Aborting this \
                     transaction, and fix this ASAP",
                    order.id
                );
                return Err(PaymentGatewayError::DatabaseError("Internal bug. Check Logs".into()));
            };
            trace!(
                "🗃️ Using {} in account {} to pay remainder of order {}",
                used_acc.current_balance,
                used_acc.id,
                order.id
            );
            total_credit -= outstanding_amount;
            used_accounts.push((used_acc.id, outstanding_amount));
            used_acc.current_balance -= outstanding_amount;
            let updated_order = orders::update_order_status(order.id, OrderStatusType::Paid, &mut tx).await?;
            debug!("🗃️ Order {} paid for during multi-account payment", order.id);
            paid_orders.push(updated_order);
        }
        // Update user balances
        trace!("🗃️ Updating balances for for used accounts: {used_accounts:?}");
        for (acc_id, amount) in &used_accounts {
            let zero = MicroTari::from(0);
            user_accounts::adjust_balances(*acc_id, zero, zero, -(*amount), &mut tx).await?;
        }
        let result = MultiAccountPayment::new(address.clone().into(), paid_orders, &used_accounts);
        Ok(result)
    }

    async fn update_payment_status(
        &self,
        txid: &str,
        status: TransferStatus,
    ) -> Result<(i64, Payment), PaymentGatewayError> {
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
        let payment = transfers::update_status(txid, status, &mut tx).await?;

        match status {
            Confirmed => user_accounts::adjust_balances(acc_id, unchanged, -amount, amount, &mut tx).await?,
            Cancelled => user_accounts::adjust_balances(acc_id, -amount, -amount, unchanged, &mut tx).await?,
            _ => unreachable!(),
        };
        debug!("🗃️ Payment [{txid}] is now {status}. Balances have been updated.");
        tx.commit().await?;
        Ok((acc_id, payment))
    }

    /// A manual order status transition from `New` to `Paid` status.
    /// This method is called by the default implementation of [`modify_status_for_order`] when the new status is
    /// `Paid`. When this happens, the following side effects occur:
    /// * A credit note for the `total_price` is created,
    async fn mark_new_order_as_paid(&self, order: Order, reason: &str) -> Result<Order, PaymentGatewayError> {
        if order.status != OrderStatusType::New {
            error!("🗃️ Order {} is not in 'New' status. Cannot call **new**_to_paid", order.id);
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        let mut tx = self.pool.begin().await?;
        let mut account = user_accounts::user_account_for_order(&order.order_id, &mut tx)
            .await?
            .ok_or_else(|| PaymentGatewayError::AccountShouldExistForOrder(order.order_id.clone()))?;
        let reason = format!("Admin credit overrode for order {}. Reason: {reason}", order.order_id);
        let note = CreditNote::new(order.customer_id.clone(), order.total_price).with_reason(reason);
        let payment = transfers::credit_note(note, &mut tx).await?;
        info!(
            "🗃️ Credit note: Customer {} received note for {} with address {}",
            order.customer_id,
            order.total_price,
            payment.sender.as_address()
        );
        // Update account ex-database. This is safe because the transaction will roll back if there's an error
        account.current_balance = account.current_balance + order.total_price;
        let updated_order = orders::try_pay_order(&account, &order, &mut tx).await?;
        tx.commit().await?;
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
        // Don't update totals, since there's a TRIGGER that effectively does this for us:
        // user_accounts::incr_order_totals(account.id, -order.total_price, -order.total_price, &mut tx).await?;
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
    /// * The [`process_order`] flow is triggered.
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

        let price = updated_order.total_price;
        let account = user_accounts::user_account_for_customer_id(&updated_order.customer_id, &mut tx)
            .await?
            .ok_or_else(|| PaymentGatewayError::AccountShouldExistForOrder(updated_order.order_id.clone()))?;
        user_accounts::incr_order_totals(account.id, price, price, &mut tx).await?;
        tx.commit().await?;
        let result = OrderChanged::new(old_order, updated_order);
        Ok(result)
    }

    /// Change the customer id for the given `order_id`. This function has several side effects:
    /// - The `customer_id` field of the order is updated in the database.
    /// - The total orders for the old and new customer are updated.
    /// - If the new customer does not exist, a new one is created.
    /// - If the order status was `Expired`, or `Cancelled`, it is **not** automatically reset to `New`. The admin must
    ///   follow up with a "change status" call to reset the order.
    ///
    /// ## Returns:
    /// - The old and new account ids; and the updated order, if it was paid for by the new account.
    ///
    /// ## Failure modes:
    /// - If the order does not exist, an error is returned.
    /// - If the order status is already `Paid`, an error is returned.
    async fn modify_customer_id_for_order(
        &self,
        order_id: &OrderId,
        new_cid: &str,
    ) -> Result<OrderMovedResult, PaymentGatewayError> {
        let update = ModifyOrderRequest::default().with_new_customer_id(new_cid);
        let mut tx = self.pool.begin().await?;
        let old_order = orders::fetch_order_by_order_id(order_id, &mut tx)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order_id.clone()))?;
        // Cannot change customer id on orders that have already been paid
        if matches!(old_order.status, OrderStatusType::Paid) {
            return Err(PaymentGatewayError::OrderModificationForbidden);
        }
        let old_acc = user_accounts::user_account_for_order(order_id, &mut tx).await?;
        if old_acc.is_none() {
            warn!("Order {order_id} does not have an associated account. This should not happen.");
            tx.rollback().await?;
            return Err(PaymentGatewayError::AccountShouldExistForOrder(order_id.clone()));
        }
        let old_account = old_acc.unwrap();
        let cust_id = Some(new_cid.into());
        let new_account_id = user_accounts::fetch_or_create_account(cust_id, None, &mut tx).await?;

        if old_account.id == new_account_id {
            debug!("🗃️ Order {order_id} is being reassigned to the same account. No action taken.");
            tx.rollback().await?;
            return Err(PaymentGatewayError::OrderModificationNoOp);
        }
        let new_order = orders::update_order(order_id, update, &mut tx).await?.ok_or_else(|| {
            error!(
                "Order {order_id} does not exist, but we fetched it within this same transaction. This should not \
                 happen. There's a data race of sorts happening here and should be sorted out."
            );
            AccountApiError::OrderDoesNotExist(order_id.clone())
        })?;
        // Order is either expired, cancelled or new by now. If expired or cancelled, we don't need to make any
        // adjustments but new orders need to be accounted for.
        if matches!(new_order.status, OrderStatusType::New) {
            user_accounts::incr_order_totals(old_account.id, -new_order.total_price, -new_order.total_price, &mut tx)
                .await?;
            debug!(
                "🗃️ We're transferring an active order {order_id} from Customer {new_cid}. Their order totals were \
                 adjusted accordingly."
            );
        }

        let mut filled_order = None;
        if let OrderStatusType::New = new_order.status {
            let new_account = user_accounts::user_account_by_id(new_account_id, &mut tx).await?.ok_or_else(|| {
                error!("Account {new_account_id} does not exist, but we just created it. This should not happen.");
                PaymentGatewayError::AccountShouldExistForOrder(order_id.clone())
            })?;
            let _ =
                user_accounts::incr_order_totals(new_account_id, new_order.total_price, new_order.total_price, &mut tx)
                    .await?;
            debug!(
                "🗃️ We've transferred an active order, {order_id}, to Customer id {new_cid}. Their order total has \
                 been adjusted accordingly"
            );
            filled_order = match orders::try_pay_order(&new_account, &new_order, &mut tx).await {
                Ok(order) => {
                    debug!("🗃️ Order {order_id} has been paid for by the new account {new_account_id}");
                    Some(order)
                },
                Err(AccountApiError::InsufficientFunds) => {
                    debug!(
                        "🗃️ There weren't enough funds to pay for order {order_id} from the new account \
                         {new_account_id} immediately, so the order remains as current"
                    );
                    None
                },
                Err(e) => return Err(e.into()),
            };
        }
        tx.commit().await?;
        let result = match filled_order {
            Some(filled_order) => OrderMovedResult::new(old_account.id, new_account_id, old_order, filled_order, true),
            None => OrderMovedResult::new(old_account.id, new_account_id, old_order, new_order, false),
        };
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

    async fn attach_order_to_address(
        &self,
        order_id: &OrderId,
        address: &TariAddress,
    ) -> Result<(UserAccount, Order), PaymentGatewayError> {
        let mut tx = self.pool.begin().await?;
        let order = orders::fetch_order_by_order_id(order_id, &mut tx)
            .await?
            .ok_or_else(|| AccountApiError::OrderDoesNotExist(order_id.clone()))?;
        let order = if order.status == OrderStatusType::Unclaimed {
            orders::update_order_status(order.id, OrderStatusType::New, &mut tx).await?
        } else {
            warn!(
                "🖇️️ Order {} is not 'Unclaimed' and we're trying to reassign the associated address. The current \
                 status is {}",
                order.order_id, order.status
            );
            order
        };
        let customer_id = order.customer_id.clone();
        let acc_id =
            match user_accounts::fetch_or_create_account(Some(customer_id), Some(address.clone()), &mut tx).await {
                Ok(id) => id,
                Err(AccountApiError::AmbiguousAccounts(info)) => {
                    info!(
                        "🖇️️ The wallet address {address} is already associated with {acc_len} account(s), but the \
                         order is associated with customer {cust} on account {o_acc}.I'm going ahead and linking \
                         {address} to account {o_acc} anyway, since it was requested.",
                        address = address,
                        acc_len = info.address_account_ids.len(),
                        cust = info.customer_id,
                        o_acc = info.customer_account_id
                    );
                    user_accounts::link_accounts(info.customer_account_id, None, Some(address.clone()), &mut tx).await?
                },
                Err(e) => return Err(e.into()),
            };
        let account = user_accounts::user_account_by_id(acc_id, &mut tx)
            .await?
            .ok_or_else(|| PaymentGatewayError::AccountShouldExistForOrder(order_id.clone()))?;
        tx.commit().await?;
        Ok((account, order))
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

    async fn fetch_customer_ids(&self, pagination: &Pagination) -> Result<Vec<String>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let ids = user_accounts::customer_ids(pagination, &mut conn).await?;
        Ok(ids)
    }

    async fn fetch_addresses(&self, pagination: &Pagination) -> Result<Vec<TariAddress>, AccountApiError> {
        let mut conn = self.pool.acquire().await?;
        let addresses = user_accounts::addresses(pagination, &mut conn).await?;
        Ok(addresses)
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

    async fn deregister_wallet(&self, _wallet_address: &TariAddress) -> Result<(), WalletManagementError> {
        let mut conn = self.pool.acquire().await?;
        wallet_auth::deregister_wallet(_wallet_address, &mut conn).await
    }

    async fn update_wallet_info(&self, _wallet: UpdateWalletInfo) -> Result<(), WalletManagementError> {
        todo!()
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
}
