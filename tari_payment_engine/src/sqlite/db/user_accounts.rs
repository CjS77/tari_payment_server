use log::{debug, error, trace};
use sqlx::{pool::PoolConnection, sqlite::SqliteRow, Row, Sqlite, SqliteConnection};
use tari_common_types::tari_address::TariAddress;
use tpg_common::MicroTari;

use crate::{
    db_types::{OrderId, UserAccount},
    order_objects::OrderQueryFilter,
    sqlite::db::{orders, transfers},
    tpe_api::account_objects::{AccountAddress, CustomerId, FullAccount, Pagination},
    traits::AccountApiError,
};

pub async fn user_account_by_id(
    account_id: i64,
    conn: &mut SqliteConnection,
) -> Result<Option<UserAccount>, AccountApiError> {
    let result = sqlx::query_as!(
        UserAccount,
        r#"
        SELECT
            id,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            total_received,
            current_pending,
            current_balance,
            total_orders,
            current_orders
        FROM user_accounts
        WHERE user_accounts.id = $1"#,
        account_id
    )
    .fetch_one(conn)
    .await;
    match result {
        Err(sqlx::Error::RowNotFound) => Ok(None),
        Err(e) => Err(e.into()),
        Ok(o) => Ok(Some(o)),
    }
}

pub async fn user_account_for_order(
    order_id: &OrderId,
    conn: &mut SqliteConnection,
) -> Result<Option<UserAccount>, AccountApiError> {
    trace!("🧑️ Fetching user account for order [{order_id}]");
    let result = sqlx::query_as!(
        UserAccount,
        r#"
        SELECT
            id,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            total_received,
            current_pending,
            current_balance,
            total_orders,
            current_orders
        FROM user_accounts
        WHERE user_accounts.id = (
            SELECT user_account_id
            FROM user_account_customer_ids INNER JOIN orders ON user_account_customer_ids.customer_id = orders
            .customer_id
            WHERE order_id = $1
            LIMIT 1
        )"#,
        order_id.0
    )
    .fetch_one(conn)
    .await;
    match result {
        Err(sqlx::Error::RowNotFound) => Ok(None),
        Err(e) => Err(e.into()),
        Ok(o) => Ok(Some(o)),
    }
}

pub async fn user_account_for_tx(txid: &str, conn: &mut SqliteConnection) -> Result<Option<UserAccount>, sqlx::Error> {
    let result = sqlx::query_as!(
        UserAccount,
        r#"
        SELECT
            id,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            total_received,
            current_pending,
            current_balance,
            total_orders,
            current_orders
        FROM user_accounts
        WHERE user_accounts.id = (
            SELECT user_account_id
            FROM user_account_address INNER JOIN payments ON user_account_address.address = payments.sender
            WHERE txid = $1
            LIMIT 1
        )"#,
        txid
    )
    .fetch_optional(conn)
    .await?;
    Ok(result)
}

/// Fetches the user account for the given customer id. If no account exists, `None` is returned.
/// Internally, the customer id (which comes from Shopify etc) is first matched with internal account ids to see if a
/// link exists. If so, the user account is fetched.
pub async fn user_account_for_customer_id(
    customer_id: &str,
    conn: &mut SqliteConnection,
) -> Result<Option<UserAccount>, AccountApiError> {
    let result = sqlx::query_as!(
        UserAccount,
        r#"
        SELECT
            user_accounts.id,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            total_received,
            current_pending,
            current_balance,
            total_orders,
            current_orders
        FROM user_accounts
        WHERE user_accounts.id = (
            SELECT user_account_id
            FROM user_account_customer_ids
            WHERE customer_id = $1
            LIMIT 1)
        "#,
        customer_id
    )
    .fetch_one(conn)
    .await;
    match result {
        Err(sqlx::Error::RowNotFound) => Ok(None),
        Err(e) => Err(e.into()),
        Ok(o) => Ok(Some(o)),
    }
}

/// Fetches the user account for the given public key. If no account exists, `None` is returned.
/// Internally, the public key is first matched with internal account ids to see if a link exists. If so, the user
/// account is fetched.
pub async fn user_account_for_address(
    address: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<Option<UserAccount>, AccountApiError> {
    let pk = address.to_hex();
    let result = sqlx::query_as!(
        UserAccount,
        r#"
        SELECT
            user_accounts.id,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            total_received,
            current_pending,
            current_balance,
            total_orders,
            current_orders
        FROM user_accounts
        WHERE user_accounts.id = (
            SELECT user_account_id
            FROM user_account_address
            WHERE address = $1
            LIMIT 1)
        "#,
        pk
    )
    .fetch_optional(conn)
    .await?;
    Ok(result)
}

/// Returns the internal account ids for the given address, in order of age (youngest first).
pub async fn fetch_account_ids_for_address(
    addr: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<Vec<i64>, AccountApiError> {
    let addr = addr.to_hex();
    let ids = sqlx::query!(
        "SELECT user_account_id FROM user_account_address WHERE address = $1 ORDER BY created_at DESC",
        addr
    )
    .fetch_all(conn)
    .await?
    .iter()
    .map(|r| r.user_account_id)
    .collect::<Vec<i64>>();
    if !ids.is_empty() {
        trace!("🧑️ Address {addr} is linked to one or more accounts");
    }
    Ok(ids)
}

/// Returns the full accounts for the given address, in order of age (youngest first).
pub async fn fetch_accounts_for_address(
    addr: &TariAddress,
    oldest_first: bool,
    conn: &mut SqliteConnection,
) -> Result<Vec<UserAccount>, AccountApiError> {
    let addr = addr.to_hex();
    let q = format!(
        "SELECT * FROM user_accounts WHERE id in (SELECT user_account_id from user_account_address WHERE address = \
         $1) ORDER BY created_at {}",
        if oldest_first { "ASC" } else { "DESC" }
    );
    let accounts = sqlx::query_as(&q).bind(addr).fetch_all(conn).await?;
    Ok(accounts)
}

/// Returns the internal account id for the given customer id, if it exists, or None if it does not exist.
async fn acc_id_for_cust_id(cid: &str, conn: &mut SqliteConnection) -> Result<Option<i64>, AccountApiError> {
    let id = sqlx::query!("SELECT user_account_id FROM user_account_customer_ids WHERE customer_id = $1 LIMIT 1", cid)
        .fetch_optional(conn)
        .await?
        .map(|r| r.user_account_id);
    if let Some(id) = id {
        debug!("🧑️ Customer_id {cid} is linked to account #{id}");
    }
    Ok(id)
}

/// Creates a new user account in the database and links it to the given customer id and/or public key.
async fn create_account_with_links(
    cid: Option<String>,
    address: Option<TariAddress>,
    tx: &mut SqliteConnection,
) -> Result<i64, AccountApiError> {
    let row = sqlx::query!("INSERT INTO user_accounts DEFAULT VALUES RETURNING id").fetch_one(&mut *tx).await?;
    let account_id = row.id;
    debug!("📝️ Created new user account with id #{account_id}");
    link_accounts(account_id, cid, address, tx).await
}

/// Links a customer id and/or address in the database with the given internal account number.
pub async fn link_accounts(
    acc_id: i64,
    cid: Option<String>,
    address: Option<TariAddress>,
    tx: &mut SqliteConnection,
) -> Result<i64, AccountApiError> {
    if let Some(cid) = cid {
        let result = sqlx::query!(
            "INSERT INTO user_account_customer_ids (user_account_id, customer_id) VALUES ($1, $2)",
            acc_id,
            cid
        )
        .execute(&mut *tx)
        .await;
        if let Err(e) = result {
            error!("Could not link customer id and user account. {e}");
        }
        debug!("🧑️ Linked user account #{acc_id} to customer_id {cid}");
    };
    if let Some(addr) = address {
        let addr = addr.to_hex();
        let result =
            sqlx::query!("INSERT INTO user_account_address (user_account_id, address) VALUES ($1, $2)", acc_id, addr,)
                .execute(tx)
                .await;
        if let Err(e) = result {
            error!("Could not link tari address and user account. {e}");
        }
        debug!("🧑️ Linked user account #{acc_id} to Tari address {addr}");
    };
    Ok(acc_id)
}

/// Fetches the user account for the given customer_id and/or public key. If both customer_id and address are
/// provided, the resulting account id must match, otherwise an error is returned (This is not an issue per se,
/// but this function doesn't have enough information to resolve the ambiguity, and so responsibility for this
/// is handed off to the caller).
///
/// If the account does not exist, one is created and the given customer id and/or public key is linked to the
/// account.
///
/// Return value: The existing or newly created account id.
pub async fn fetch_or_create_account(
    cust_id: Option<String>,
    address: Option<TariAddress>,
    conn: &mut SqliteConnection,
) -> Result<i64, AccountApiError> {
    if cust_id.is_none() && address.is_none() {
        return Err(AccountApiError::QueryError(
            "🧑️ Nothing to do. Both cid and address are None. I don't want to create an orphan account".to_string(),
        ));
    }

    trace!("🧑️ Fetching or creating user account for customer_id {cust_id:?} and address {address:?}");
    let cid_is_linked = match &cust_id {
        Some(cid) => acc_id_for_cust_id(cid, &mut *conn).await?,
        None => None,
    };
    trace!(
        "🧑️ Customer_id is {} the database at account #{}",
        cid_is_linked.map_or("NOT in", |_| "IN"),
        cid_is_linked.map_or(-1, |id| id),
    );

    let address_accounts = match &address {
        Some(pk) => fetch_account_ids_for_address(pk, &mut *conn).await?,
        None => vec![],
    };

    trace!(
        "🧑️ Address {} an account in the database",
        if address_accounts.is_empty() { "does NOT have" } else { "HAS" },
    );

    let id = match (cid_is_linked, address_accounts.is_empty()) {
        (Some(acc_cid), false) => {
            if address_accounts.contains(&acc_cid) {
                Ok(acc_cid)
            } else {
                Err(AccountApiError::ambiguous(address.unwrap(), address_accounts, cust_id.unwrap(), acc_cid))
            }
        },
        (Some(account_id), true) => link_accounts(account_id, None, address, &mut *conn).await,
        // Link the customer id to the most recent account associated with this address
        (None, false) => link_accounts(address_accounts[0], cust_id, None, &mut *conn).await,
        (None, true) => create_account_with_links(cust_id, address, &mut *conn).await,
    }?;
    Ok(id)
}

// Sets the current balance for the given account id, rather than adding a delta to it
pub async fn update_user_balance(
    account_id: i64,
    balance: MicroTari,
    conn: &mut SqliteConnection,
) -> Result<(), AccountApiError> {
    let _ = sqlx::query!(
        r#"UPDATE user_accounts SET
       current_balance = $1,
       updated_at = CURRENT_TIMESTAMP
       WHERE id = $2
       "#,
        balance,
        account_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn adjust_balances(
    account_id: i64,
    received_delta: MicroTari,
    pending_delta: MicroTari,
    balance_delta: MicroTari,
    conn: &mut SqliteConnection,
) -> Result<(), AccountApiError> {
    let d_rec = received_delta.value();
    let d_pend = pending_delta.value();
    let d_bal = balance_delta.value();
    let _ = sqlx::query!(
        r#"UPDATE user_accounts SET
       current_balance = current_balance + $1,
       total_received = total_received + $2,
       current_pending = current_pending + $3,
       updated_at = CURRENT_TIMESTAMP
       WHERE id = $4
       "#,
        d_bal,
        d_rec,
        d_pend,
        account_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Increments the total and current order counts for the given account id.
/// Returns the new total order count.
///
/// # Arguments
/// account_id - The internal account id
/// delta_total - The amount to increment the total values of orders made by this customer
/// delta_current - The amount to increment the current/pending order total for this customer
pub async fn incr_order_totals(
    account_id: i64,
    delta_total: MicroTari,
    delta_current: MicroTari,
    conn: &mut SqliteConnection,
) -> Result<MicroTari, AccountApiError> {
    let value_total = delta_total.value();
    let value_current = delta_current.value();
    let result = sqlx::query!(
        r#"UPDATE user_accounts SET
       total_orders = total_orders + $1,
       current_orders = current_orders + $2,
       updated_at = CURRENT_TIMESTAMP
       WHERE id = $3
       RETURNING total_orders
       "#,
        value_total,
        value_current,
        account_id
    )
    .fetch_one(conn)
    .await?;
    let new_total = MicroTari::from(result.total_orders);
    Ok(new_total)
}

pub async fn fetch_addresses_for_account(
    account_id: i64,
    conn: &mut SqliteConnection,
) -> Result<Vec<AccountAddress>, AccountApiError> {
    let addresses: Vec<AccountAddress> =
        sqlx::query_as(r#"SELECT address, created_at, updated_at FROM user_account_address WHERE user_account_id = ?"#)
            .bind(account_id)
            .fetch_all(conn)
            .await?;
    Ok(addresses)
}

pub async fn fetch_customer_ids_for_account(
    account_id: i64,
    conn: &mut SqliteConnection,
) -> Result<Vec<CustomerId>, AccountApiError> {
    let cust_ids = sqlx::query_as!(
        CustomerId,
        r#"SELECT customer_id,
    created_at as "created_at: chrono::DateTime<chrono::Utc>",
    updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
    FROM user_account_customer_ids WHERE user_account_id = ?"#,
        account_id
    )
    .fetch_all(conn)
    .await?;
    Ok(cust_ids)
}

pub(crate) async fn history_for_id(
    id: i64,
    conn: &mut PoolConnection<Sqlite>,
) -> Result<Option<FullAccount>, AccountApiError> {
    let Some(account) = user_account_by_id(id, conn).await? else {
        return Ok(None);
    };
    let addresses = fetch_addresses_for_account(id, conn).await?;
    let customer_ids = fetch_customer_ids_for_account(id, conn).await?;
    let mut all_payments = vec![];
    for address in &addresses {
        let mut payments = transfers::fetch_payments_for_address(address.address.as_address(), conn).await?;
        all_payments.append(&mut payments);
    }
    let mut all_orders = vec![];
    for cust_id in &customer_ids {
        let query = OrderQueryFilter::default().with_customer_id(cust_id.customer_id.clone());
        let mut orders = orders::search_orders(query, conn).await?;
        all_orders.append(&mut orders);
    }
    let result = FullAccount::new(account)
        .with_addresses(addresses)
        .with_customer_ids(customer_ids)
        .with_orders(all_orders)
        .with_payments(all_payments);
    Ok(Some(result))
}

pub(crate) async fn creditors(conn: &mut SqliteConnection) -> Result<Vec<UserAccount>, AccountApiError> {
    let accounts = sqlx::query_as!(
        UserAccount,
        r#"
        SELECT
            id,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            total_received,
            current_pending,
            current_balance,
            total_orders,
            current_orders
        FROM user_accounts
        WHERE current_balance > 0 OR current_pending > 0"#
    )
    .fetch_all(conn)
    .await?;
    Ok(accounts)
}

pub(crate) async fn customer_ids(
    pagination: &Pagination,
    conn: &mut SqliteConnection,
) -> Result<Vec<String>, AccountApiError> {
    let rows = with_pagination("SELECT customer_id FROM user_account_customer_ids", pagination, conn).await?;
    let customer_ids = rows.into_iter().map(|r| r.get("customer_id")).collect::<Vec<String>>();
    Ok(customer_ids)
}

pub(crate) async fn addresses(
    pagination: &Pagination,
    conn: &mut SqliteConnection,
) -> Result<Vec<TariAddress>, AccountApiError> {
    let rows = with_pagination("SELECT address FROM user_account_address", pagination, conn).await?;
    let addresses = rows.into_iter().filter_map(|r| TariAddress::from_hex(r.get("address")).ok()).collect();
    Ok(addresses)
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
