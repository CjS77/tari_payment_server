use crate::PaymentGatewayDatabase;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrderManagerError<B: PaymentGatewayDatabase> {
    #[error("Database error: {0}")]
    DatabaseError(B::Error),
    #[error("Order already exists. Order.id = {0:?}")]
    OrderAlreadyExists(i64),
}
