use log::{debug, error, trace};
use sqlx::SqliteConnection;
use tari_common_types::tari_address::TariAddress;

use crate::{
    db::sqlite::SqliteDatabaseError,
    db_types::{MicroTari, NewOrder, NewPayment, OrderId, UserAccount},
};

pub async fn user_account_by_id(
    account_id: i64,
    conn: &mut SqliteConnection,
) -> Result<Option<UserAccount>, SqliteDatabaseError> {
    let result = sqlx::query_as!(
        UserAccount,
        r#"
        SELECT
            id,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            total_received,
            total_pending,
            current_balance,
            total_orders
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
) -> Result<Option<UserAccount>, SqliteDatabaseError> {
    trace!("🧑️ Fetching user account for order [{order_id}]");
    let result = sqlx::query_as!(
        UserAccount,
        r#"
        SELECT
            id,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            total_received,
            total_pending,
            current_balance,
            total_orders
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

pub async fn user_account_for_tx(
    txid: &str,
    conn: &mut SqliteConnection,
) -> Result<Option<UserAccount>, SqliteDatabaseError> {
    let result = sqlx::query_as!(
        UserAccount,
        r#"
        SELECT
            id,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            total_received,
            total_pending,
            current_balance,
            total_orders
        FROM user_accounts
        WHERE user_accounts.id = (
            SELECT user_account_id
            FROM user_account_public_keys INNER JOIN payments ON user_account_public_keys.public_key = payments.sender
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
) -> Result<Option<UserAccount>, SqliteDatabaseError> {
    let result = sqlx::query_as!(
        UserAccount,
        r#"
        SELECT
            user_accounts.id,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            total_received,
            total_pending,
            current_balance,
            total_orders
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
    public_key: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<Option<UserAccount>, SqliteDatabaseError> {
    let pk = public_key.to_hex();
    let result = sqlx::query_as!(
        UserAccount,
        r#"
        SELECT
            user_accounts.id,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
            total_received,
            total_pending,
            current_balance,
            total_orders
        FROM user_accounts
        WHERE user_accounts.id = (
            SELECT user_account_id
            FROM user_account_public_keys
            WHERE public_key = $1
            LIMIT 1)
        "#,
        pk
    )
    .fetch_optional(conn)
    .await?;
    Ok(result)
}

/// Returns the internal account id for the given public key, if it exists, or None if it does not exist.
async fn acc_id_for_pubkey(pk: &TariAddress, conn: &mut SqliteConnection) -> Result<Option<i64>, SqliteDatabaseError> {
    let pk = pk.to_hex();
    let id = sqlx::query!("SELECT user_account_id FROM user_account_public_keys WHERE public_key = $1 LIMIT 1", pk)
        .fetch_optional(conn)
        .await?
        .map(|r| r.user_account_id);
    if let Some(id) = id {
        trace!("🧑️ Public key {pk} is linked to account #{id}");
    }
    Ok(id)
}

/// Returns the internal account id for the given customer id, if it exists, or None if it does not exist.
async fn acc_id_for_cust_id(cid: &str, conn: &mut SqliteConnection) -> Result<Option<i64>, SqliteDatabaseError> {
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
    pk: Option<TariAddress>,
    tx: &mut SqliteConnection,
) -> Result<i64, SqliteDatabaseError> {
    let row = sqlx::query!("INSERT INTO user_accounts DEFAULT VALUES RETURNING id").fetch_one(&mut *tx).await?;
    let account_id = row.id;
    debug!("📝️ Created new user account with id #{account_id}");
    link_accounts(account_id, cid, pk, tx).await
}

/// Links a customer id and/or public key in the database with the given internal account number.
async fn link_accounts(
    acc_id: i64,
    cid: Option<String>,
    pk: Option<TariAddress>,
    tx: &mut SqliteConnection,
) -> Result<i64, SqliteDatabaseError> {
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
    if let Some(pk) = pk {
        let addr = pk.to_hex();
        let result = sqlx::query!(
            "INSERT INTO user_account_public_keys (user_account_id, public_key) VALUES ($1, $2)",
            acc_id,
            addr,
        )
        .execute(tx)
        .await;
        if let Err(e) = result {
            error!("Could not link tari address and user account. {e}");
        }
        debug!("🧑️ Linked user account #{acc_id} to Tari address {pk}");
    };
    Ok(acc_id)
}

/// Fetches the user account for the given customer_id and/or public key. If both customer_id and public_key are
/// provided, the resulting account id must match, otherwise an error is returned.
///
/// If the account does not exist, one is created and the given customer id and/or public key is linked to the
/// account.
///
/// Return value: The existing or newly created account id.
pub async fn fetch_or_create_account(
    cust_id: Option<String>,
    pubkey: Option<TariAddress>,
    conn: &mut SqliteConnection,
) -> Result<i64, SqliteDatabaseError> {
    if cust_id.is_none() && pubkey.is_none() {
        return Err(SqliteDatabaseError::AccountCreationError(
            "🧑️ Nothing to do. Both cid and pubkey are None. I don't want to create an orphan account".to_string(),
        ));
    }

    trace!("🧑️ Fetching or creating user account for customer_id {cust_id:?} and public_key {pubkey:?}");
    let cid_is_linked = match &cust_id {
        Some(cid) => acc_id_for_cust_id(cid, &mut *conn).await?,
        None => None,
    };
    trace!(
        "🧑️ Customer_id is {} the database at account #{}",
        cid_is_linked.map_or("NOT in", |_| "IN"),
        cid_is_linked.map_or(-1, |id| id),
    );

    let pk_is_linked = match &pubkey {
        Some(pk) => acc_id_for_pubkey(pk, &mut *conn).await?,
        None => None,
    };

    trace!(
        "🧑️ Public key is {} the database at account #{}",
        pk_is_linked.map_or("NOT in", |_| "IN"),
        pk_is_linked.map_or(-1, |id| id),
    );

    let id = match (cid_is_linked, pk_is_linked) {
        (Some(acc_cid), Some(acc_pk)) => {
            if acc_cid == acc_pk {
                Ok(acc_cid)
            } else {
                Err(SqliteDatabaseError::AccountCreationError(
                    "🧑️ Customer_id and public_key are linked to different accounts".to_string(),
                ))
            }
        },
        (Some(account_id), None) => link_accounts(account_id, None, pubkey, &mut *conn).await,
        (None, Some(account_id)) => link_accounts(account_id, cust_id, None, &mut *conn).await,
        (None, None) => create_account_with_links(cust_id, pubkey, &mut *conn).await,
    }?;
    Ok(id)
}

async fn try_match_order_to_payments(
    order: &NewOrder,
    conn: &mut SqliteConnection,
) -> Result<Option<i64>, SqliteDatabaseError> {
    trace!("🧑️ Trying to match order {order:?} to existing payments.");
    if order.address.is_none() {
        return Ok(None);
    }
    let pubkey = order.address.as_ref().unwrap();
    acc_id_for_pubkey(pubkey, conn).await
}

async fn try_match_payment_to_orders(
    payment: &NewPayment,
    _conn: &mut SqliteConnection,
) -> Result<i64, SqliteDatabaseError> {
    trace!("🧑️ Trying to match payment {payment:?} to existing orders.");
    todo!()
}

pub async fn update_user_balance(
    account_id: i64,
    balance: MicroTari,
    conn: &mut SqliteConnection,
) -> Result<(), SqliteDatabaseError> {
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
) -> Result<(), SqliteDatabaseError> {
    let d_rec = received_delta.value();
    let d_pend = pending_delta.value();
    let d_bal = balance_delta.value();
    let _ = sqlx::query!(
        r#"UPDATE user_accounts SET
       current_balance = current_balance + $1,
       total_received = total_received + $2,
       total_pending = total_pending + $3,
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

pub async fn incr_total_orders(
    account_id: i64,
    delta: MicroTari,
    conn: &mut SqliteConnection,
) -> Result<MicroTari, SqliteDatabaseError> {
    let value = delta.value();
    let result = sqlx::query!(
        r#"UPDATE user_accounts SET
       total_orders = total_orders + $1,
       updated_at = CURRENT_TIMESTAMP
       WHERE id = $2
       RETURNING total_orders
       "#,
        value,
        account_id
    )
    .fetch_one(conn)
    .await?;
    let new_total = MicroTari::from(result.total_orders);
    Ok(new_total)
}
