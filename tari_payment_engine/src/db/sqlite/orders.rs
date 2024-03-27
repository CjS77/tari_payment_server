use log::{debug, trace};
use sqlx::{QueryBuilder, SqliteConnection};

use crate::{
    db::{common::InsertOrderResult, sqlite::SqliteDatabaseError},
    db_types::{NewOrder, Order, OrderId, OrderStatusType, OrderUpdate},
};

pub async fn idempotent_insert(
    order: NewOrder,
    conn: &mut SqliteConnection,
) -> Result<InsertOrderResult, SqliteDatabaseError> {
    let result = match order_exists(&order.order_id, conn).await? {
        Some(id) => InsertOrderResult::AlreadyExists(id),
        None => insert_order(order, conn).await?,
    };
    Ok(result)
}

/// Inserts a new order into the database using the given connection. This is not atomic. You can embedd this call
/// inside a transaction if you need to ensure atomicity, and pass `&mut *tx` as the connection argument.
async fn insert_order(order: NewOrder, conn: &mut SqliteConnection) -> Result<InsertOrderResult, SqliteDatabaseError> {
    let record = sqlx::query!(
        r#"
            INSERT INTO orders (
                order_id,
                customer_id,
                memo,
                total_price,
                currency
            ) VALUES ($1, $2, $3, $4, $5)
            RETURNING id;
        "#,
        order.order_id,
        order.customer_id,
        order.memo,
        order.total_price,
        order.currency,
    )
    .fetch_one(conn)
    .await?;
    // The DB should trigger an automatic status entry for the order
    Ok(InsertOrderResult::Inserted(record.id))
}

/// Returns the last entry i the orders table for the corresponding `order_id`
pub async fn fetch_order_by_order_id(
    order_id: &OrderId,
    conn: &mut SqliteConnection,
) -> Result<Option<Order>, SqliteDatabaseError> {
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
        Err(e) => Err(e.into()),
        Ok(o) => Ok(Some(o)),
    }
}

/// Checks whether the order with the given `OrderId` already exists in the database. If it does exist, the `id` of the
/// order is returned. If it does not exist, `None` is returned.
pub async fn order_exists(order_id: &OrderId, conn: &mut SqliteConnection) -> Result<Option<i64>, SqliteDatabaseError> {
    fetch_order_by_order_id(order_id, conn).await.map(|o| o.map(|o| o.id))
}

#[derive(Debug, Clone, Default)]
pub struct OrderQueryFilter {
    memo: Option<String>,
    order_id: Option<OrderId>,
    account_id: Option<i64>,
    currency: Option<String>,
    statuses: Vec<OrderStatusType>,
}

impl OrderQueryFilter {
    pub fn with_memo(mut self, memo: String) -> Self {
        self.memo = Some(memo);
        self
    }

    pub fn with_order_id(mut self, order_id: OrderId) -> Self {
        self.order_id = Some(order_id);
        self
    }

    pub fn with_account_id(mut self, account_id: i64) -> Self {
        self.account_id = Some(account_id);
        self
    }

    pub fn with_currency(mut self, currency: String) -> Self {
        self.currency = Some(currency);
        self
    }

    pub fn with_status(mut self, status: OrderStatusType) -> Self {
        self.statuses.push(status);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.memo.is_none() &&
            self.order_id.is_none() &&
            self.account_id.is_none() &&
            self.currency.is_none() &&
            self.statuses.is_empty()
    }
}

/// Fetches orders according to criteria specified in the `OrderQueryFilter`
///
/// Resulting orders are ordered by `created_at` in ascending order
pub async fn fetch_orders(
    query: OrderQueryFilter,
    conn: &mut SqliteConnection,
) -> Result<Vec<Order>, SqliteDatabaseError> {
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
        where_clause.push("memo = ");
        where_clause.push_bind_unseparated(memo);
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
    if let Some(currency) = query.currency {
        where_clause.push("currency = ");
        where_clause.push_bind_unseparated(currency);
    }
    if !query.statuses.is_empty() {
        let mut statuses = vec![];
        query.statuses.iter().for_each(|s| {
            statuses.push(format!("'{s}'"));
        });
        let status_clause = statuses.join(",");
        where_clause.push(format!("status IN ({status_clause})"));
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
) -> Result<(), SqliteDatabaseError> {
    let status = status.to_string();
    let _ =
        sqlx::query!("UPDATE orders SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2", status, order_id)
            .execute(conn)
            .await?;
    Ok(())
}

pub(crate) async fn update_order(
    id: &OrderId,
    update: OrderUpdate,
    conn: &mut SqliteConnection,
) -> Result<(), SqliteDatabaseError> {
    if update.is_empty() {
        debug!("📝️ No fields to update for order {id}. Update request skipped.");
        return Ok(());
    }
    let mut builder = QueryBuilder::new("UPDATE orders SET updated_at = CURRENT_TIMESTAMP,");
    let mut set_clause = builder.separated(", ");
    if let Some(status) = update.status {
        set_clause.push("status = ");
        set_clause.push_bind_unseparated(status.to_string());
    }
    if let Some(memo) = update.memo {
        set_clause.push("memo = ");
        set_clause.push_bind_unseparated(memo);
    }
    if let Some(total_price) = update.total_price {
        set_clause.push("total_price = ");
        set_clause.push_bind_unseparated(total_price);
    }
    if let Some(currency) = update.currency {
        set_clause.push("currency = ");
        set_clause.push_bind_unseparated(currency);
    }
    builder.push(" WHERE order_id = ");
    builder.push_bind(id.as_str());
    trace!("📝️ Executing query: {}", builder.sql());
    let res = builder.build().execute(conn).await?;
    trace!("📝️ Result of update_order: {res:?}");
    Ok(())
}
