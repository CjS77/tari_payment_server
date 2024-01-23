use crate::address_extractor::{extract_order_number_from_memo, extract_public_key_from_memo};
use crate::db::sqlite::SqliteDatabaseError;
use crate::db_types::{MicroTari, NewOrder, NewPayment, OrderId, UserAccount};
use log::{debug, error, trace};
use sqlx::SqliteConnection;
use tari_common_types::tari_address::TariAddress;

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
    let result = sqlx::query_as!(UserAccount,
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
    ).fetch_one(conn).await;
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
    let result = sqlx::query_as!(UserAccount,
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
        )"#,txid
    ).fetch_optional(conn).await?;
    Ok(result)
}

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

pub async fn user_account_for_public_key(
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

async fn acc_id_for_pubkey(
    pk: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<Option<i64>, SqliteDatabaseError> {
    let pk = pk.to_hex();
    let id = sqlx::query!(
        "SELECT user_account_id FROM user_account_public_keys WHERE public_key = $1 LIMIT 1",
        pk
    )
    .fetch_optional(conn)
    .await?
    .map(|r| r.user_account_id);
    if let Some(id) = id {
        trace!("🧑️ Public key {pk} is linked to account #{id}");
    }
    Ok(id)
}

async fn acc_id_for_cust_id(
    cid: &str,
    conn: &mut SqliteConnection,
) -> Result<Option<i64>, SqliteDatabaseError> {
    let id = sqlx::query!(
        "SELECT user_account_id FROM user_account_customer_ids WHERE customer_id = $1 LIMIT 1",
        cid
    )
    .fetch_optional(conn)
    .await?
    .map(|r| r.user_account_id);
    if let Some(id) = id {
        debug!("🧑️ Customer_id {cid} is linked to account #{id}");
    }
    Ok(id)
}

async fn create_account_with_links(
    cid: Option<String>,
    pk: Option<TariAddress>,
    tx: &mut SqliteConnection,
) -> Result<i64, SqliteDatabaseError> {
    let row = sqlx::query!("INSERT INTO user_accounts DEFAULT VALUES RETURNING id")
        .fetch_one(&mut *tx)
        .await?;
    let account_id = row.id;
    debug!("📝️ Created new user account with id #{account_id}");
    link_accounts(account_id, cid, pk, tx).await
}

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
pub async fn fetch_or_create_account(
    order: Option<NewOrder>,
    payment: Option<NewPayment>,
    conn: &mut SqliteConnection,
) -> Result<i64, SqliteDatabaseError> {
    if order.is_none() && payment.is_none() {
        return Err(SqliteDatabaseError::AccountCreationError(
            "🧑️ Nothing to do. Both order and payment are None. I don't want to create an orphan account".to_string(),
        ));
    }

    let cust_id = order.as_ref().map(|o| o.customer_id.clone());
    let pubkey = payment.as_ref().map(|p| p.sender.clone());

    let cid_is_linked = match &cust_id {
        Some(cid) => acc_id_for_cust_id(cid, &mut *conn).await?,
        None => None,
    };

    let pk_is_linked = match &pubkey {
        Some(pk) => acc_id_for_pubkey(pk, &mut *conn).await?,
        None => None,
    };

    let id = match (cid_is_linked, pk_is_linked) {
        (Some(cid), Some(pk)) => {
            if cid == pk {
                Ok(cid)
            } else {
                Err(SqliteDatabaseError::AccountCreationError(
                    "🧑️ Customer_id and public_key are linked to different accounts".to_string(),
                ))
            }
        }
        (Some(account_id), None) => link_accounts(account_id, None, pubkey, &mut *conn).await,
        (None, Some(account_id)) => link_accounts(account_id, cust_id, None, &mut *conn).await,
        (None, None) => {
            // The user account does not appear to exist, but this still may be a payment that should be matched
            // with an order, or vice versa.
            trace!("🧑️ Trying to match order and payment to existing accounts.");
            if let Some(id) = try_match_order_to_payments(order, &mut *conn).await? {
                link_accounts(id, cust_id, pubkey, conn).await?;
                trace!("🧑️ Order matched to payment. Account id: {id}");
                Ok(id)
            } else if let Some(id) = try_match_payment_to_orders(payment, &mut *conn).await? {
                link_accounts(id, cust_id, pubkey, conn).await?;
                trace!("🧑️ Payment matched to order. Account id: {id}");
                Ok(id)
            } else {
                create_account_with_links(cust_id, pubkey, &mut *conn).await
            }
        }
    }?;
    Ok(id)
}

async fn try_match_order_to_payments(
    order: Option<NewOrder>,
    conn: &mut SqliteConnection,
) -> Result<Option<i64>, SqliteDatabaseError> {
    if order.is_none() {
        return Ok(None);
    }
    let order = order.unwrap();
    // Currently, the only way to match an order to a payment is by memo.
    if order.memo.is_none() {
        return Ok(None);
    }
    let memo = order.memo.unwrap();
    trace!("🧑️ Matching order memo [{memo}] to payments..");
    let Some(pubkey) = extract_public_key_from_memo(&memo) else {
        return Ok(None);
    };
    trace!("🧑️ Order memo matched to public key {pubkey}");
    acc_id_for_pubkey(&pubkey, conn).await
}

async fn try_match_payment_to_orders(
    payment: Option<NewPayment>,
    conn: &mut SqliteConnection,
) -> Result<Option<i64>, SqliteDatabaseError> {
    if payment.is_none() {
        return Ok(None);
    }
    let payment = payment.unwrap();
    // Currently, the only way to match a payment to an order is by memo.
    if payment.memo.is_none() {
        return Ok(None);
    }
    let memo = payment.memo.unwrap();
    trace!("🧑️ Matching payment memo [{memo}] to orders..");
    let Some(order_id) = extract_order_number_from_memo(&memo) else {
        return Ok(None);
    };
    trace!("🧑️ Payment memo matched to order id {order_id}");
    let account = user_account_for_order(&order_id, conn).await?;
    Ok(account.map(|a| a.id))
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
