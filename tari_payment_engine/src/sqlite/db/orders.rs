use chrono::Duration;
use log::{debug, trace};
use sqlx::{sqlite::SqliteRow, FromRow, QueryBuilder, SqliteConnection};
use tari_common_types::tari_address::TariAddress;

use crate::{
    db_types::{NewOrder, Order, OrderId, OrderStatusType},
    order_objects::{ModifyOrderRequest, OrderQueryFilter},
    traits::PaymentGatewayError,
};

/// Inserts the order into the database, returning `false` in the second parameter if the order already exists.
pub async fn idempotent_insert(
    order: NewOrder,
    conn: &mut SqliteConnection,
) -> Result<(Order, bool), PaymentGatewayError> {
    let inserted = match fetch_order_by_order_id(&order.order_id, conn).await? {
        Some(order) => (order, false),
        None => {
            let order = insert_order(order, conn).await?;
            debug!("📝️ Order [{}] inserted with id {}", order.order_id, order.id);
            (order, true)
        },
    };
    Ok(inserted)
}

/// Inserts a new order into the database using the given connection. This is not atomic. You can embed this call
/// inside a transaction if you need to ensure atomicity, and pass `&mut *tx` as the connection argument.
///
/// If a Tari Address is provided, and it already exists in the database, the order status is set to 'New'.
/// If the address is not found in the database, or if it is not provided, the order status is set to 'Unclaimed'.
async fn insert_order(order: NewOrder, conn: &mut SqliteConnection) -> Result<Order, PaymentGatewayError> {
    let order = sqlx::query_as(
        r#"
            INSERT INTO orders (
                order_id,
                alt_id,
                customer_id,
                memo,
                total_price,
                original_price,
                currency,
                created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *;
        "#,
    )
    .bind(order.order_id)
    .bind(order.alt_order_id)
    .bind(order.customer_id)
    .bind(order.memo)
    .bind(order.total_price.value())
    .bind(order.original_price)
    .bind(order.currency)
    .bind(order.created_at)
    .fetch_one(conn)
    .await?;
    // The DB should trigger an automatic status entry for the order
    Ok(order)
}

/// Returns the last entry in the orders table for the corresponding `order_id`
pub async fn fetch_order_by_order_id(
    order_id: &OrderId,
    conn: &mut SqliteConnection,
) -> Result<Option<Order>, sqlx::Error> {
    let order =
        sqlx::query_as("SELECT * FROM orders WHERE order_id = $1").bind(order_id.as_str()).fetch_optional(conn).await?;
    Ok(order)
}

/// Returns the last entry in the orders table for the corresponding `alt_order_id`
pub async fn fetch_order_by_alt_id(alt: &OrderId, conn: &mut SqliteConnection) -> Result<Option<Order>, sqlx::Error> {
    let order =
        sqlx::query_as("SELECT * FROM orders WHERE alt_id = $1").bind(alt.as_str()).fetch_optional(conn).await?;
    Ok(order)
}

/// Returns the last entry in the orders table for the corresponding `order_id` or `alt_id`.
/// If an order_id and alt_id match on different orders, then the one matching the order_id is returned.
pub async fn fetch_order_by_id_or_alt(id: &OrderId, conn: &mut SqliteConnection) -> Result<Option<Order>, sqlx::Error> {
    let order = sqlx::query_as("SELECT * FROM orders WHERE order_id = $1 or alt_id = $1 ORDER BY alt_id limit 1")
        .bind(id.as_str())
        .fetch_optional(conn)
        .await?;
    Ok(order)
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
    SELECT * FROM orders
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
    if let Some(alt_id) = query.alt_id {
        where_clause.push("alt_id = ");
        where_clause.push_bind_unseparated(alt_id.to_string());
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
    id: i64,
    status: OrderStatusType,
    conn: &mut SqliteConnection,
) -> Result<Order, PaymentGatewayError> {
    let status = status.to_string();
    let result: Option<Order> =
        sqlx::query_as("UPDATE orders SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2 RETURNING *")
            .bind(status)
            .bind(id)
            .fetch_optional(conn)
            .await?;
    result.ok_or(PaymentGatewayError::OrderIdNotFound(id))
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
    let mut builder = QueryBuilder::new("UPDATE orders SET updated_at = CURRENT_TIMESTAMP, ");
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
    if let Some(original_price) = update.new_original_price {
        set_clause.push("original_price = ");
        set_clause.push_bind_unseparated(original_price);
    }
    if let Some(currency) = update.new_currency {
        set_clause.push("currency = ");
        set_clause.push_bind_unseparated(currency);
    }
    if let Some(cust_id) = update.new_customer_id {
        set_clause.push("customer_id = ");
        set_clause.push_bind_unseparated(cust_id);
    }
    builder.push(" WHERE order_id = ");
    builder.push_bind(id.as_str());
    builder.push(" RETURNING *");
    trace!("📝️ Executing query: {}", builder.sql());
    let res = builder.build().fetch_optional(conn).await?.map(|row: SqliteRow| Order::from_row(&row)).transpose()?;
    trace!("📝️ Result of update_order: {res:?}");
    Ok(res)
}

pub(crate) async fn expire_orders(
    status: OrderStatusType,
    limit: Duration,
    conn: &mut SqliteConnection,
) -> Result<Vec<Order>, PaymentGatewayError> {
    let rows = sqlx::query_as(
        format!(
            "UPDATE orders SET updated_at = CURRENT_TIMESTAMP, status = 'Expired' WHERE status = '{status}' AND \
             (unixepoch(CURRENT_TIMESTAMP) - unixepoch(updated_at)) > {} RETURNING *;",
            limit.num_seconds()
        )
        .as_str(),
    )
    .fetch_all(conn)
    .await?;
    Ok(rows)
}

/// Fetches all payable orders for the given address. A payable order is one that is "New" or "Unclaimed"
/// i.e. it has not been paid and is associated with the address.
pub(crate) async fn fetch_payable_orders_for_address(
    address: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<Vec<Order>, PaymentGatewayError> {
    let result: Vec<Order> = sqlx::query_as(
        r#"
        SELECT
            orders.id as id,
            order_id,
            alt_id,
            orders.customer_id as customer_id,
            memo,
            total_price,
            original_price,
            currency,
            orders.created_at as created_at,
            orders.updated_at as updated_at,
            status
        FROM orders JOIN address_customer_id_link ON orders.customer_id = address_customer_id_link.customer_id
        WHERE
         status in ('New', 'Unclaimed') AND
         address = $1"#,
    )
    .bind(address.to_base58())
    .fetch_all(conn)
    .await?;
    Ok(result)
}
