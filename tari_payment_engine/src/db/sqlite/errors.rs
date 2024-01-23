use thiserror::Error;

#[derive(Debug, Error)]
pub enum SqliteDatabaseError {
    #[error("Database connection error: {0}")]
    DriverError(#[from] sqlx::Error),
    #[error("Database query error: {0}")]
    QueryError(String),
    #[error("Could not create new user account: {0}")]
    AccountCreationError(String),
    #[error("Account not found: {0}")]
    AccountNotFound(i64),
    #[error("There is no account associated with tx: {0}")]
    AccountNotLinkedWithTransaction(String),
    #[error("Cannot process duplicate order #{0}")]
    DuplicateOrder(i64),
    #[error("Cannot process duplicate transfer #{0}")]
    DuplicatePayment(String),
    #[error("Could not update payment status: {0}")]
    PaymentStatusUpdateError(String),
}
