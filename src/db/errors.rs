use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    DriverError(#[from] sqlx::Error),
    #[error("Database query error: {0}")]
    QueryError(String),
}

#[derive(Debug, Error)]
#[error("Could not convert shopify order into a new order. {0}.")]
pub struct OrderConversionError(pub String);
