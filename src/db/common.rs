use crate::db::errors::DatabaseError;
use crate::db::models::NewOrder;
use crate::order_matcher::messages::TransferReceived;

pub enum InsertResult {
    Inserted,
    AlreadyExists,
}

#[cfg(feature = "sqlite")]
pub type DbPool = sqlx::SqlitePool;
#[cfg(feature = "postgres")]
pub type DbPool = sqlx::PgPool;

pub struct Database {
    pool: DbPool,
}

#[cfg(feature = "sqlite")]
use super::sqlite;
#[cfg(feature = "sqlite")]
impl Database {
    pub async fn new() -> Result<Self, DatabaseError> {
        let pool = sqlite::new_pool().await?;
        Ok(Self { pool })
    }

    pub async fn insert_order(&self, order: NewOrder) -> Result<InsertResult, DatabaseError> {
        sqlite::orders::idempotent_insert(order, &self.pool).await
    }

    pub async fn insert_transfer(
        &self,
        transfer: TransferReceived,
    ) -> Result<InsertResult, DatabaseError> {
        sqlite::transfers::idempotent_insert(transfer, &self.pool).await
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }
}
