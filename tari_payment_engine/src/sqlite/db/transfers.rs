use chrono::Utc;
use sqlx::SqliteConnection;
use tari_common_types::tari_address::TariAddress;

use crate::{
    db_types::{CreditNote, NewPayment, Payment, TransferStatus},
    helpers::create_dummy_address_for_cust_id,
    traits::PaymentGatewayError,
};

pub async fn idempotent_insert(
    transfer: NewPayment,
    conn: &mut SqliteConnection,
) -> Result<Payment, PaymentGatewayError> {
    let txid = transfer.txid.clone();
    let address = transfer.sender.as_address().to_hex();
    let payment = sqlx::query_as(
        r#"
            INSERT INTO payments (txid, sender, amount, memo) VALUES ($1, $2, $3, $4)
            RETURNING *;
        "#,
    )
    .bind(transfer.txid)
    .bind(address)
    .bind(transfer.amount)
    .bind(transfer.memo)
    .fetch_one(conn)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(err) if err.is_unique_violation() => PaymentGatewayError::PaymentAlreadyExists(txid),
        _ => PaymentGatewayError::from(e),
    })?;
    Ok(payment)
}

/// Issues a credit note against the customer id. Since payments require a sender address,
/// a dummy address is created that is unique to the customer id and easily identifiable as a dummy address.
///
/// If the credit note is successfully issued, the address of the dummy address is returned.
pub async fn credit_note(note: CreditNote, conn: &mut SqliteConnection) -> Result<Payment, PaymentGatewayError> {
    let timestamp = Utc::now().timestamp();
    let txid = format!("credit_note_{}:{}:{timestamp}", note.customer_id, note.amount);
    let address = create_dummy_address_for_cust_id(&note.customer_id);
    let hex_addr = address.to_hex();
    let memo = format!("Credit note: {}", note.reason.unwrap_or("No reason given".into()));
    let payment = sqlx::query_as(
        r#"
            INSERT INTO payments (txid, sender, amount, memo, payment_type, status)
            VALUES ($1, $2, $3, $4, 'Manual', 'Confirmed') RETURNING *;
        "#,
    )
    .bind(txid.clone())
    .bind(hex_addr)
    .bind(note.amount)
    .bind(memo)
    .fetch_one(conn)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(err) if err.is_unique_violation() => PaymentGatewayError::PaymentAlreadyExists(txid),
        _ => PaymentGatewayError::from(e),
    })?;
    Ok(payment)
}

pub async fn update_status(
    txid: &str,
    status: TransferStatus,
    conn: &mut SqliteConnection,
) -> Result<Payment, PaymentGatewayError> {
    let status = status.to_string();
    let payment = sqlx::query_as("UPDATE payments SET status = $1 WHERE txid = $2 RETURNING *")
        .bind(status)
        .bind(txid)
        .fetch_optional(conn)
        .await?
        .ok_or(PaymentGatewayError::PaymentStatusUpdateError(format!("Payment for {txid} does not exist")))?;
    Ok(payment)
}

pub async fn fetch_payment(txid: &str, conn: &mut SqliteConnection) -> Result<Option<Payment>, PaymentGatewayError> {
    let payment = sqlx::query_as(r#"SELECT * FROM payments WHERE txid = ?"#).bind(txid).fetch_optional(conn).await?;
    Ok(payment)
}

pub async fn fetch_payments_for_address(
    address: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<Vec<Payment>, sqlx::Error> {
    let payments =
        sqlx::query_as(r#"SELECT * FROM payments WHERE sender = ?"#).bind(address.to_hex()).fetch_all(conn).await?;
    Ok(payments)
}
