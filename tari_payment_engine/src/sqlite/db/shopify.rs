use chrono::Utc;
use log::debug;
use sqlx::{Error as SqlxError, SqliteConnection};

use crate::{
    shopify_types::{NewShopifyAuthorization, ShopifyAuthorization},
    traits::ShopifyAuthorizationError,
};

pub async fn insert_new_shopify_auth(
    auth: NewShopifyAuthorization,
    conn: &mut SqliteConnection,
) -> Result<ShopifyAuthorization, ShopifyAuthorizationError> {
    let result: ShopifyAuthorization = sqlx::query_as(
        r#"INSERT INTO shopify_transactions
        (id, order_id, amount, currency, test, captured)
        VALUES (?, ?, ?, ?, ?, ?)
        RETURNING *;
        "#,
    )
    .bind(auth.id)
    .bind(auth.order_id)
    .bind(auth.amount)
    .bind(auth.currency)
    .bind(auth.test)
    .bind(auth.captured)
    .fetch_one(conn)
    .await
    .map_err(|e| match e {
        SqlxError::RowNotFound => ShopifyAuthorizationError::NotFound(auth.id, auth.order_id),
        SqlxError::Database(e) if e.is_unique_violation() => {
            ShopifyAuthorizationError::AlreadyExists(auth.id, auth.order_id)
        },
        e => ShopifyAuthorizationError::DatabaseError(e.to_string()),
    })?;
    Ok(result)
}

pub async fn fetch_auth_by_order_id(
    oid: i64,
    conn: &mut SqliteConnection,
) -> Result<Vec<ShopifyAuthorization>, ShopifyAuthorizationError> {
    let result = sqlx::query_as("SELECT * FROM shopify_transactions WHERE order_id = ?;")
        .bind(oid)
        .fetch_all(conn)
        .await
        .map_err(|e| ShopifyAuthorizationError::DatabaseError(e.to_string()))?;
    Ok(result)
}

/// Set all authorizations for the given order id to the given status.
/// Returns the number of rows affected.
pub async fn capture_auth(
    order_id: i64,
    capture: bool,
    conn: &mut SqliteConnection,
) -> Result<Vec<ShopifyAuthorization>, ShopifyAuthorizationError> {
    let result = sqlx::query_as(
        "UPDATE shopify_transactions SET captured = $1, updated_at = $2 WHERE id = $3 AND captured != $1RETURNING *;",
    )
    .bind(capture)
    .bind(Utc::now())
    .bind(order_id)
    .fetch_all(conn)
    .await
    .map_err(|e| ShopifyAuthorizationError::DatabaseError(e.to_string()))?;
    debug!("Set captured = {capture} for order {order_id}");
    Ok(result)
}
