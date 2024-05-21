use thiserror::Error;

use crate::PaymentGatewayDatabase;

#[derive(Debug, Error)]
pub enum OrderManagerError<B: PaymentGatewayDatabase> {
    #[error("Database error: {0}")]
    DatabaseError(B::Error),
    #[error("Order already exists. Order.id = {0:?}")]
    OrderAlreadyExists(i64),
}

#[derive(Debug, Clone, Error)]
pub enum AuthApiError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Nonce is not strictly increasing.")]
    InvalidNonce,
    #[error("Tari address not found")]
    AddressNotFound,
    #[error("User requested at least {0} roles that are not allowed")]
    RoleNotAllowed(usize),
    #[error("The requested role does not exist")]
    RoleNotFound,
}

#[derive(Debug, Clone, Error)]
pub enum AccountApiError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("User error constructing query: {0}")]
    QueryError(String),
}
