use crate::db::common::InsertResult;
use crate::db::errors::DatabaseError;
use crate::db::models::{NewOrder, Order};
use sqlx::SqlitePool;

pub async fn idempotent_insert(
    order: NewOrder,
    pool: &SqlitePool,
) -> Result<InsertResult, DatabaseError> {
    let exists = order_exists(&order, pool).await?;
    if exists {
        Ok(InsertResult::AlreadyExists)
    } else {
        insert_order(order, pool).await
    }
}

async fn insert_order(order: NewOrder, pool: &SqlitePool) -> Result<InsertResult, DatabaseError> {
    let timestamp = chrono::Utc::now().timestamp();
    sqlx::query!(
        r#"
            INSERT INTO orders (
                order_id,
                customer_id,
                memo,
                total_price,
                currency,
                timestamp
            ) VALUES ($1, $2, $3, $4, $5, $6);
        "#,
        order.order_id,
        order.customer_id,
        order.memo,
        order.total_price,
        order.currency,
        timestamp,
    )
    .execute(pool)
    .await?;
    Ok(InsertResult::Inserted)
}

pub async fn fetch_last_entry_for_order(
    order: &NewOrder,
    pool: &SqlitePool,
) -> Result<Option<Order>, DatabaseError> {
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
                timestamp as "timestamp: chrono::DateTime<chrono::Utc>"
            FROM orders
            WHERE order_id = $1
            ORDER BY id DESC
            LIMIT 1;
        "#,
        order.order_id
    )
    .fetch_one(pool)
    .await;
    match order {
        Err(sqlx::Error::RowNotFound) => Ok(None),
        Err(e) => Err(e.into()),
        Ok(o) => Ok(Some(o)),
    }
}

pub async fn order_exists(order: &NewOrder, pool: &SqlitePool) -> Result<bool, DatabaseError> {
    let last_order = fetch_last_entry_for_order(order, pool).await?;
    match last_order {
        None => Ok(false),
        Some(o) => Ok(order.is_equivalent(&o)),
    }
}
