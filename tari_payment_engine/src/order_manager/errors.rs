use crate::db_types::Role;
use crate::PaymentGatewayDatabase;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrderManagerError<B: PaymentGatewayDatabase> {
    #[error("Database error: {0}")]
    DatabaseError(B::Error),
    #[error("Order already exists. Order.id = {0:?}")]
    OrderAlreadyExists(i64),
}

#[derive(Debug, Error)]
pub enum AuthApiError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Nonce is not strictly increasing")]
    InvalidNonce,
    #[error("Public key not found")]
    PubkeyNotFound,
    #[error("User requested a role that is not allowed")]
    RoleNotAllowed(Role),
}
