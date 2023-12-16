use crate::db::errors::DatabaseError;
use crate::db::InsertResult;
use crate::order_matcher::messages::TransferReceived;
use sqlx::SqlitePool;

pub async fn idempotent_insert(
    transfer: TransferReceived,
    pool: &SqlitePool,
) -> Result<InsertResult, DatabaseError> {
    let exists = transfer_exists(&transfer, pool).await?;
    if exists {
        Ok(InsertResult::AlreadyExists)
    } else {
        insert_transfer(transfer, pool).await
    }
}

async fn insert_transfer(
    transfer: TransferReceived,
    pool: &SqlitePool,
) -> Result<InsertResult, DatabaseError> {
    #[allow(clippy::cast_possible_wrap)]  // Not an issue for a few hundred years..
    let height = transfer.block_height as i64;
    sqlx::query!(
        r#"
            INSERT INTO payments (
                timestamp,
                block_height,
                sender,
                receiver,
                amount,
                memo
            ) VALUES ($1, $2, $3, $4, $5, $6);
        "#,
        transfer.timestamp,
        height,
        transfer.sender,
        transfer.receiver,
        transfer.amount,
        transfer.memo,
    )
    .execute(pool)
    .await?;
    Ok(InsertResult::Inserted)
}

pub async fn transfer_exists(
    transfer: &TransferReceived,
    pool: &SqlitePool,
) -> Result<bool, DatabaseError> {
    #[allow(clippy::cast_possible_wrap)]  // Not an issue for a few hundred years..
    let height = transfer.block_height as i64;
    sqlx::query!(
        r#"
            SELECT EXISTS (
                SELECT 1 FROM payments
                WHERE timestamp = $1
                AND block_height = $2
                AND sender = $3
                AND receiver = $4
                AND amount = $5
                AND memo = $6
            ) as "exists!: bool";
        "#,
        transfer.timestamp,
        height,
        transfer.sender,
        transfer.receiver,
        transfer.amount,
        transfer.memo,
    )
    .fetch_one(pool)
    .await
    .map(|row| row.exists)
    .map_err(|e| DatabaseError::QueryError(e.to_string()))
}
