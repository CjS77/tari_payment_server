use log::{debug, trace};
use sqlx::{sqlite::SqliteRow, FromRow, QueryBuilder, SqliteConnection};

use crate::{
    db_types::{MicroTari, NewOrder, Order, OrderId, OrderStatusType, UserAccount},
    order_objects::{ModifyOrderRequest, OrderQueryFilter},
    sqlite::db::user_accounts,
    traits::{AccountApiError, PaymentGatewayError},
};

pub async fn idempotent_insert(order: NewOrder, conn: &mut SqliteConnection) -> Result<i64, PaymentGatewayError> {
    match order_exists(&order.order_id, conn).await? {
        Some(id) => Err(PaymentGatewayError::OrderAlreadyExists(id)),
        None => insert_order(order, conn).await,
    }
}

/// Inserts a new order into the database using the given connection. This is not atomic. You can embedd this call
/// inside a transaction if you need to ensure atomicity, and pass `&mut *tx` as the connection argument.
async fn insert_order(order: NewOrder, conn: &mut SqliteConnection) -> Result<i64, PaymentGatewayError> {
    let record = sqlx::query!(
        r#"
            INSERT INTO orders (
                order_id,
                customer_id,
                memo,
                total_price,
                currency,
                created_at
            ) VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id;
        "#,
        order.order_id,
        order.customer_id,
        order.memo,
        order.total_price,
        order.currency,
        order.created_at
    )
    .fetch_one(conn)
    .await?;
    // The DB should trigger an automatic status entry for the order
    Ok(record.id)
}

/// Returns the last entry in the orders table for the corresponding `order_id`
pub async fn fetch_order_by_order_id(
    order_id: &OrderId,
    conn: &mut SqliteConnection,
) -> Result<Option<Order>, sqlx::Error> {
    let order = sqlx::query_as!(
        Order,
        r#"
            SELECT
                id,
                order_id,
                customer_id,
                memo,
                total_price,
                currency,
                created_at as "created_at: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
                status
            FROM orders
            WHERE order_id = $1
            ORDER BY id DESC
            LIMIT 1;
        "#,
        order_id
    )
    .fetch_one(conn)
    .await;
    match order {
        Err(sqlx::Error::RowNotFound) => Ok(None),
        Err(e) => Err(e),
        Ok(o) => Ok(Some(o)),
    }
}

/// Checks whether the order with the given `OrderId` already exists in the database. If it does exist, the `id` of the
/// order is returned. If it does not exist, `None` is returned.
pub async fn order_exists(order_id: &OrderId, conn: &mut SqliteConnection) -> Result<Option<i64>, PaymentGatewayError> {
    let order = fetch_order_by_order_id(order_id, conn).await?;
    Ok(order.map(|o| o.id))
}

/// Fetches orders according to criteria specified in the `OrderQueryFilter`
///
/// Resulting orders are ordered by `created_at` in ascending order
pub async fn search_orders(query: OrderQueryFilter, conn: &mut SqliteConnection) -> Result<Vec<Order>, sqlx::Error> {
    let mut builder = QueryBuilder::new(
        r#"
    SELECT id, order_id, customer_id, memo, total_price, currency, created_at, updated_at, status FROM orders
    "#,
    );
    if !query.is_empty() {
        builder.push("WHERE ");
    }
    let mut where_clause = builder.separated(" AND ");
    if let Some(memo) = query.memo {
        where_clause.push("memo LIKE ");
        where_clause.push_bind_unseparated(format!("%{memo}%"));
    }
    if let Some(order_id) = query.order_id {
        where_clause.push("order_id = ");
        where_clause.push_bind_unseparated(order_id.to_string());
    }
    if let Some(id) = query.account_id {
        where_clause.push("customer_id in (SELECT customer_id FROM user_account_customer_ids WHERE user_account_id = ");
        where_clause.push_bind_unseparated(id);
        where_clause.push_unseparated(")");
    }
    if let Some(cid) = query.customer_id {
        where_clause.push("customer_id=");
        where_clause.push_bind_unseparated(cid);
    }
    if let Some(currency) = query.currency {
        where_clause.push("currency=");
        where_clause.push_bind_unseparated(currency);
    }
    if query.status.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
        let mut statuses = vec![];
        query.status.as_ref().unwrap().iter().for_each(|s| {
            statuses.push(format!("'{s}'"));
        });
        let status_clause = statuses.join(",");
        where_clause.push(format!("status IN ({status_clause})"));
    }
    if let Some(since) = query.since {
        where_clause.push("created_at >= ");
        where_clause.push_bind_unseparated(since);
    }
    if let Some(until) = query.until {
        where_clause.push("created_at <= ");
        where_clause.push_bind_unseparated(until);
    }
    builder.push(" ORDER BY created_at ASC");

    trace!("📝️ Executing query: {}", builder.sql());
    let query = builder.build_query_as::<Order>();
    let orders = query.fetch_all(conn).await?;
    trace!("Result of fetch_orders: {:?}", orders.len());
    Ok(orders)
}

pub(crate) async fn update_order_status(
    order_id: i64,
    status: OrderStatusType,
    conn: &mut SqliteConnection,
) -> Result<Order, PaymentGatewayError> {
    let status = status.to_string();
    let result: Option<Order> =
        sqlx::query_as("UPDATE orders SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2 RETURNING *")
            .bind(status)
            .bind(order_id)
            .fetch_optional(conn)
            .await?;
    result.ok_or(PaymentGatewayError::OrderIdNotFound(order_id))
}

pub(crate) async fn update_order(
    id: &OrderId,
    update: ModifyOrderRequest,
    conn: &mut SqliteConnection,
) -> Result<Option<Order>, PaymentGatewayError> {
    if update.is_empty() {
        debug!("📝️ No fields to update for order {id}. Update request skipped.");
        return Err(PaymentGatewayError::OrderModificationNoOp);
    }
    let mut builder = QueryBuilder::new("UPDATE orders SET updated_at = CURRENT_TIMESTAMP,");
    let mut set_clause = builder.separated(", ");
    if let Some(status) = update.new_status {
        set_clause.push("status = ");
        set_clause.push_bind_unseparated(status.to_string());
    }
    if let Some(memo) = update.new_memo {
        set_clause.push("memo = ");
        set_clause.push_bind_unseparated(memo);
    }
    if let Some(total_price) = update.new_total_price {
        set_clause.push("total_price = ");
        set_clause.push_bind_unseparated(total_price);
    }
    if let Some(currency) = update.new_currency {
        set_clause.push("currency = ");
        set_clause.push_bind_unseparated(currency);
    }
    builder.push(" WHERE order_id = ");
    builder.push_bind(id.as_str());
    builder.push("RETURNING *");
    trace!("📝️ Executing query: {}", builder.sql());
    let res = builder.build().fetch_optional(conn).await?.map(|row: SqliteRow| Order::from_row(&row)).transpose()?;
    trace!("📝️ Result of update_order: {res:?}");
    Ok(res)
}

pub(crate) async fn try_pay_order(
    account: &UserAccount,
    order: &Order,
    conn: &mut SqliteConnection,
) -> Result<Order, AccountApiError> {
    let mut current_balance = account.current_balance;
    if current_balance < order.total_price {
        return Err(AccountApiError::InsufficientFunds);
    }
    let acc_id = account.id;
    current_balance -= order.total_price;
    let order = update_order_status(order.id, OrderStatusType::Paid, conn).await.map_err(|e| match e {
        PaymentGatewayError::DatabaseError(s) => AccountApiError::DatabaseError(s),
        _ => unreachable!("Unexpected error type: {e}"),
    })?;
    trace!("📝️ Order #{} of {} marked as paid", order.id, order.total_price);
    user_accounts::update_user_balance(account.id, current_balance, conn).await?;
    trace!("Account {acc_id} balance updated from {} to {current_balance}", account.current_balance);
    user_accounts::incr_order_totals(account.id, MicroTari::from(0), -order.total_price, conn).await?;
    trace!("📝️ Adjusted account #{acc_id} orders outstanding by {}.", order.total_price);
    Ok(order)
}
