use super::{db_url, new_pool, orders, transfers, user_accounts, SqliteDatabaseError};
use crate::db::common::{AccountManagement, OrderManagement, PaymentGatewayDatabase};
use crate::db::sqlite::orders::OrderQueryFilter;

use crate::db_types::{
    MicroTari, NewOrder, NewPayment, Order, OrderId, OrderStatusType, OrderUpdate, TransferStatus,
    UserAccount,
};
use crate::{InsertOrderResult, InsertPaymentResult};
use log::*;
use sqlx::SqlitePool;
use std::fmt::Debug;
use tari_common_types::tari_address::TariAddress;

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
    type Error = SqliteDatabaseError;

    fn url(&self) -> &str {
        self.url.as_str()
    }

    async fn fetch_or_create_account(
        &self,
        order: Option<NewOrder>,
        payment: Option<NewPayment>,
    ) -> Result<i64, Self::Error> {
        let mut conn = self.pool.acquire().await?;
        user_accounts::fetch_or_create_account(order, payment, &mut conn).await
    }

    async fn process_new_order_for_customer(&self, order: NewOrder) -> Result<i64, Self::Error> {
        let mut tx = self.pool.begin().await?;
        let price = order.total_price;
        let _cid = order.customer_id.clone();
        let id = match orders::idempotent_insert(order.clone(), &mut tx).await {
            Ok(InsertOrderResult::Inserted(id)) => Ok(id),
            Ok(InsertOrderResult::AlreadyExists(id)) => {
                Err(SqliteDatabaseError::DuplicateOrder(id))
            }
            Err(e) => Err(e),
        }?;
        debug!(
            "🗃️ Order #{} has been saved in the DB with id {id}",
            order.order_id
        );
        let account_id =
            user_accounts::fetch_or_create_account(Some(order.clone()), None, &mut tx).await?;
        user_accounts::incr_total_orders(account_id, price, &mut tx).await?;
        tx.commit().await?;
        Ok(account_id)
    }

    /// Takes a new payment, and in a single atomic transaction,
    /// * calls `save_payment` to store the payment in the database. If the payment already exists,
    ///   nothing further is done.
    /// * The payment is marked as `Unconfirmed`
    /// * creates a new account for the public key if one does not already exist
    /// * Adds the payment amount to the account's total received, and total pending
    /// Returns the account id for the public key.
    async fn process_new_payment_for_pubkey(
        &self,
        payment: NewPayment,
    ) -> Result<i64, Self::Error> {
        let mut tx = self.pool.begin().await?;
        let txid = match transfers::idempotent_insert(payment.clone(), &mut tx).await {
            Ok(InsertPaymentResult::Inserted(id)) => Ok(id),
            Ok(InsertPaymentResult::AlreadyExists(_id)) => {
                Err(SqliteDatabaseError::DuplicatePayment(payment.txid.clone()))
            }
            Err(e) => Err(e),
        }?;
        debug!("🗃️ Transfer {txid} received from [{}]", payment.sender);
        let acc_id =
            user_accounts::fetch_or_create_account(None, Some(payment.clone()), &mut tx).await?;
        user_accounts::adjust_balances(
            acc_id,
            payment.amount,
            payment.amount,
            MicroTari::from(0),
            &mut tx,
        )
        .await?;
        debug!(
            "🗃️ Transfer {txid} processed. {} credited to pending account",
            payment.amount
        );
        tx.commit().await?;
        Ok(acc_id)
    }

    async fn fetch_payable_orders(&self, account_id: i64) -> Result<Vec<Order>, Self::Error> {
        let mut tx = self.pool.begin().await?;
        let account = user_accounts::user_account_by_id(account_id, &mut tx)
            .await?
            .ok_or_else(|| SqliteDatabaseError::AccountNotFound(account_id))?;
        let query = OrderQueryFilter::default()
            .with_account_id(account_id)
            .with_status(OrderStatusType::New);
        let unpaid_orders = orders::fetch_orders(query, &mut tx).await?;
        let balance = account.current_balance;
        trace!(
            "🗃️ Account #{account_id} has {} unpaid orders and a balance of {}.",
            unpaid_orders.len(),
            balance
        );
        let (paid_orders, _new_balance) = unpaid_orders.into_iter().fold(
            (vec![], balance),
            |(mut orders, mut balance), order| {
                if balance >= order.total_price {
                    balance -= order.total_price;
                    orders.push(order);
                }
                (orders, balance)
            },
        );
        tx.commit().await?;
        Ok(paid_orders)
    }

    async fn try_pay_orders(
        &self,
        account_id: i64,
        orders: &[Order],
    ) -> Result<Vec<Order>, Self::Error> {
        let mut tx = self.pool.begin().await?;
        let account = user_accounts::user_account_by_id(account_id, &mut tx)
            .await?
            .ok_or_else(|| SqliteDatabaseError::AccountNotFound(account_id))?;
        let mut new_balance = account.current_balance;
        let mut result = Vec::with_capacity(orders.len());
        for order in orders {
            if new_balance >= order.total_price {
                new_balance -= order.total_price;
                orders::update_order_status(order.id, OrderStatusType::Paid, &mut tx).await?;
                trace!(
                    "🗃️ Order #{} of {} marked as paid",
                    order.id,
                    order.total_price
                );
                result.push(order.clone());
            }
        }
        user_accounts::update_user_balance(account_id, new_balance, &mut tx).await?;
        trace!(
            "Account {account_id} balance updated from {} to {new_balance}",
            account.current_balance
        );
        tx.commit().await?;
        Ok(result)
    }

    async fn update_payment_status(
        &self,
        txid: &str,
        status: TransferStatus,
    ) -> Result<Option<i64>, Self::Error> {
        let mut tx = self.pool.begin().await?;
        let payment = transfers::fetch_payment(txid, &mut tx).await?;
        if payment.is_none() {
            return Err(SqliteDatabaseError::PaymentStatusUpdateError(format!(
                "Payment {txid} not found"
            )));
        }
        let payment = payment.unwrap();
        let old_status = payment.status;
        use TransferStatus::*;
        if old_status == status {
            debug!("🗃️ Payment {txid} already has status {status}. No action to take");
            return Ok(None);
        }
        if old_status != Received {
            error!("🗃️ Payment {txid} cannot be transitioned from {old_status} to {status}.\
                If there is a valid use case, perform a manual adjustment now and submit a ticket so that it can be \
                handled properly in the future.");
            return Err(SqliteDatabaseError::PaymentStatusUpdateError(format!(
                "Payment {txid} has status {status} instead of 'Received'"
            )));
        }

        let account = match user_accounts::user_account_for_tx(txid, &mut tx).await {
            Ok(Some(acc)) => Ok(acc),
            Ok(None) => Err(SqliteDatabaseError::AccountNotLinkedWithTransaction(
                txid.to_string(),
            )),
            Err(e) => Err(e),
        }?;
        let acc_id = account.id;
        let unchanged = MicroTari::from(0);
        let amount = payment.amount;
        transfers::update_status(txid, status, &mut tx).await?;

        match status {
            Confirmed => {
                user_accounts::adjust_balances(acc_id, unchanged, -amount, amount, &mut tx).await?
            }
            Cancelled => {
                user_accounts::adjust_balances(acc_id, -amount, -amount, unchanged, &mut tx).await?
            }
            _ => unreachable!(),
        };
        debug!("🗃️ Payment [{txid}] is now {status}. Balances have been updated.");
        tx.commit().await?;
        Ok(Some(acc_id))
    }

    async fn update_order(&self, id: &OrderId, update: OrderUpdate) -> Result<(), Self::Error> {
        let mut tx = self.pool.acquire().await?;
        trace!("🗃️ Order {id} updating with new values: {update:?}");
        orders::update_order(id, update, &mut tx).await?;
        trace!("🗃️ Order {id} has been updated.");
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        self.pool.close().await;
        Ok(())
    }
}

impl AccountManagement for SqliteDatabase {
    type Error = SqliteDatabaseError;

    async fn fetch_user_account(
        &self,
        account_id: i64,
    ) -> Result<Option<UserAccount>, Self::Error> {
        let mut conn = self.pool.acquire().await?;
        user_accounts::user_account_by_id(account_id, &mut conn).await
    }

    /// Fetches the user account for the given order id. A user account must have already been created for this account.
    /// If no account is found, `None` will be returned.
    ///
    /// Alternatively, you can search through the memo fields of payments to find a matching order id by calling
    /// [`search_for_user_account_by_memo`].
    async fn fetch_user_account_for_order(
        &self,
        order_id: &OrderId,
    ) -> Result<Option<UserAccount>, Self::Error> {
        let mut conn = self.pool.acquire().await?;
        user_accounts::user_account_for_order(order_id, &mut conn).await
    }

    /// Searches through the memo fields of payments to find a matching order id. If no account is found, `None` will be
    /// returned.
    ///
    /// The `memo_match` is a string that is used to search for a matching order id using `LIKE`.
    /// For example, `format!("%Order id: [{order_id}]%)` will match any memo that contains the order id."
    async fn search_for_user_account_by_memo(
        &self,
        memo_match: &str,
    ) -> Result<Option<i64>, Self::Error> {
        let mut conn = self.pool.acquire().await?;
        user_accounts::search_for_user_account_by_order_id_in_memo(memo_match, &mut conn).await
    }

    async fn fetch_user_account_for_customer_id(
        &self,
        customer_id: &str,
    ) -> Result<Option<UserAccount>, Self::Error> {
        let mut conn = self.pool.acquire().await?;
        user_accounts::user_account_for_customer_id(customer_id, &mut conn).await
    }

    async fn fetch_user_account_for_pubkey(
        &self,
        pubkey: &TariAddress,
    ) -> Result<Option<UserAccount>, Self::Error> {
        let mut conn = self.pool.acquire().await?;
        user_accounts::user_account_for_public_key(pubkey, &mut conn).await
    }
}

impl OrderManagement for SqliteDatabase {
    type Error = SqliteDatabaseError;

    async fn order_by_id(&self, oid: &OrderId) -> Result<Option<Order>, Self::Error> {
        let mut conn = self.pool.acquire().await?;
        orders::fetch_order_by_order_id(oid, &mut conn).await
    }
}

impl SqliteDatabase {
    /// Creates a new database API object
    pub async fn new() -> Result<Self, SqliteDatabaseError> {
        let url = db_url();
        SqliteDatabase::new_with_url(url.as_str()).await
    }

    pub async fn new_with_url(url: &str) -> Result<Self, SqliteDatabaseError> {
        trace!("Creating new database connection pool with url {url}");
        let pool = new_pool(url).await?;
        let url = url.to_string();
        Ok(Self { url, pool })
    }

    /// Retrieve the last entry for the corresponding `order_id` from the orders table. If no entry
    /// exists, `None` will be returned.
    pub async fn order_by_order_id(
        &self,
        order_id: &OrderId,
    ) -> Result<Option<Order>, SqliteDatabaseError> {
        let mut conn = self.pool.acquire().await?;
        orders::fetch_order_by_order_id(order_id, &mut conn).await
    }
    /// Returns a reference to the database connection pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Fetches all orders from the database that match the given memo field.
    /// The match is fuzzy. As long as the memo _contains_ the given string, the order will be returned.
    pub async fn fetch_orders_by_memo(
        &self,
        memo: &str,
    ) -> Result<Vec<Order>, SqliteDatabaseError> {
        let where_clause = OrderQueryFilter::default()
            .with_memo(memo.trim().to_string())
            .with_status(OrderStatusType::Paid)
            .with_status(OrderStatusType::New)
            .with_currency("XTR".to_string());
        let mut conn = self.pool.acquire().await?;
        let orders = orders::fetch_orders(where_clause, &mut conn).await?;
        Ok(orders)
    }
}
