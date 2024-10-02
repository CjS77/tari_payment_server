use sqlx::{sqlite::SqliteRow, Row, SqliteConnection};
use tari_common_types::tari_address::TariAddress;

use crate::{
    db_types::{
        AddressBalance,
        CustomerOrderBalance,
        CustomerOrders,
        NewSettlementJournalEntry,
        Order,
        OrderId,
        SettlementJournalEntry,
    },
    tpe_api::account_objects::Pagination,
    traits::AccountApiError,
};

/// Links an address to a customer id. This function is idempotent due to a uniqueness constraint on the DB table.
pub(crate) async fn link_address_to_customer(
    address: &TariAddress,
    customer_id: &str,
    conn: &mut SqliteConnection,
) -> Result<(), AccountApiError> {
    let address = address.to_base58();
    sqlx::query!("INSERT INTO address_customer_id_link (address, customer_id) VALUES (?, ?)", address, customer_id,)
        .execute(conn)
        .await?;
    Ok(())
}

pub(crate) async fn balances_for_customer_id(
    customer_id: &str,
    conn: &mut SqliteConnection,
) -> Result<Vec<AddressBalance>, AccountApiError> {
    let addresses: Vec<AddressBalance> = sqlx::query_as(
        r#"
    SELECT * FROM address_balance
    WHERE address in (SELECT address from address_customer_id_link WHERE customer_id = $1)
    ORDER BY last_update DESC
    "#,
    )
    .bind(customer_id)
    .fetch_all(conn)
    .await?;
    Ok(addresses)
}

pub(crate) async fn balances_for_order_id(
    order_id: &OrderId,
    conn: &mut SqliteConnection,
) -> Result<Vec<AddressBalance>, AccountApiError> {
    let addresses: Vec<AddressBalance> = sqlx::query_as(
        r#"
    SELECT * FROM address_balance
    WHERE address in (SELECT sender from payments WHERE order_id = $1)
    ORDER BY last_update DESC
    "#,
    )
    .bind(order_id.as_str())
    .fetch_all(conn)
    .await?;
    Ok(addresses)
}

pub(crate) async fn fetch_address_balance(
    address: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<AddressBalance, AccountApiError> {
    let balance: Option<AddressBalance> = sqlx::query_as("SELECT * FROM address_balance WHERE address = $1")
        .bind(address.to_base58())
        .fetch_optional(conn)
        .await?;
    Ok(balance.unwrap_or_else(|| AddressBalance::new(address.clone())))
}

pub(crate) async fn insert_settlement(
    settlement: NewSettlementJournalEntry,
    conn: &mut SqliteConnection,
) -> Result<SettlementJournalEntry, AccountApiError> {
    let result = sqlx::query_as(
        r#"
    INSERT INTO settlement_journal (order_id, payment_address, amount, settlement_type)
    VALUES (?, ?, ?, ?)
    RETURNING *
    "#,
    )
    .bind(settlement.order_id)
    .bind(settlement.payment_address.as_base58())
    .bind(settlement.amount)
    .bind(settlement.settlement_type)
    .fetch_one(conn)
    .await?;
    Ok(result)
}

pub(crate) async fn settlements_for_address(
    address: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<Vec<SettlementJournalEntry>, AccountApiError> {
    let settlements: Vec<SettlementJournalEntry> =
        sqlx::query_as("SELECT * FROM settlement_journal WHERE payment_address = $1")
            .bind(address.to_base58())
            .fetch_all(conn)
            .await?;
    Ok(settlements)
}

pub(crate) async fn settlements_for_customer_id(
    customer_id: &str,
    conn: &mut SqliteConnection,
) -> Result<Vec<SettlementJournalEntry>, AccountApiError> {
    let settlements: Vec<SettlementJournalEntry> = sqlx::query_as(
        r#"
    SELECT * FROM settlement_journal WHERE order_id in (
      SELECT order_id FROM orders WHERE customer_id = $1 AND status = 'Paid'
    )"#,
    )
    .bind(customer_id)
    .fetch_all(conn)
    .await?;
    Ok(settlements)
}

pub(crate) async fn orders_for_address(
    address: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<Vec<Order>, AccountApiError> {
    let accounts: Vec<Order> = sqlx::query_as(
        r#"
    SELECT
        orders.id as id,
        orders.order_id as order_id,
        orders.customer_id as customer_id,
        orders.memo as memo,
        orders.total_price as total_price,
        orders.original_price as original_price,
        orders.currency as currency,
        orders.created_at as created_at,
        orders.updated_at as updated_at,
        orders.status as status
    FROM orders JOIN address_customer_id_link ON orders.customer_id = address_customer_id_link.customer_id
    WHERE address = $1
    "#,
    )
    .bind(address.to_base58())
    .fetch_all(conn)
    .await?;
    Ok(accounts)
}

pub(crate) async fn creditors(conn: &mut SqliteConnection) -> Result<Vec<CustomerOrders>, AccountApiError> {
    let addresses: Vec<CustomerOrders> =
        sqlx::query_as("SELECT * FROM customer_order_balance WHERE status = 'New' AND total_orders > 0")
            .fetch_all(conn)
            .await?;
    Ok(addresses)
}

pub(crate) async fn customer_order_balance(
    cust_id: &str,
    conn: &mut SqliteConnection,
) -> Result<CustomerOrderBalance, AccountApiError> {
    let orders: Vec<CustomerOrders> = sqlx::query_as("SELECT * FROM customer_order_balance WHERE customer_id = $1")
        .bind(cust_id)
        .fetch_all(conn)
        .await?;
    let balances = CustomerOrderBalance::new(&orders);
    Ok(balances)
}

pub(crate) async fn customer_ids(
    pagination: &Pagination,
    conn: &mut SqliteConnection,
) -> Result<Vec<String>, AccountApiError> {
    let rows =
        with_pagination("SELECT DISTINCT customer_id FROM orders ORDER BY customer_id", pagination, conn).await?;
    let customer_ids = rows.into_iter().map(|r| r.get("customer_id")).collect::<Vec<String>>();
    Ok(customer_ids)
}

pub(crate) async fn addresses(
    pagination: &Pagination,
    conn: &mut SqliteConnection,
) -> Result<Vec<TariAddress>, AccountApiError> {
    let rows = with_pagination("SELECT DISTINCT sender FROM payments ORDER BY sender ASC", pagination, conn).await?;
    let addresses = rows.into_iter().filter_map(|r| TariAddress::from_base58(r.get("sender")).ok()).collect();
    Ok(addresses)
}

pub(crate) async fn customer_ids_for_address(
    address: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<Vec<String>, AccountApiError> {
    let rows = sqlx::query("SELECT customer_id FROM address_customer_id_link WHERE address = $1")
        .bind(address.to_base58())
        .fetch_all(conn)
        .await?;
    let customer_ids = rows.into_iter().map(|r| r.get("customer_id")).collect::<Vec<String>>();
    Ok(customer_ids)
}

async fn with_pagination(
    q: &str,
    pagination: &Pagination,
    conn: &mut SqliteConnection,
) -> Result<Vec<SqliteRow>, AccountApiError> {
    let limit = match pagination.count {
        Some(_) => " LIMIT ?",
        None => "",
    };
    let offset = match pagination.count {
        Some(_) => " OFFSET ?",
        None => "",
    };
    let q = format!("{q} {limit} {offset}");
    let mut query = sqlx::query(&q);
    if let Some(count) = pagination.count {
        query = query.bind(count);
    }
    if let Some(offset) = pagination.offset {
        query = query.bind(offset);
    }
    let rows = query.fetch_all(conn).await?;
    Ok(rows)
}
