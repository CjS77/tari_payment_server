use crate::db_types::OrderStatus;

use crate::db_types::{MicroTari, OrderId, PublicKey};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct OrderCreated {
    pub created_at: DateTime<Utc>,
    pub order_id: OrderId,
    pub customer_id: String,
    pub memo: String, // This is used to match the order with the payment
    pub total_price: MicroTari,
    pub currency: String,
}

#[derive(Debug, Clone)]
pub struct OrderStatusMessage(pub OrderStatus);

pub struct PaymentReceived {
    /// The time the payment was received
    pub timestamp: DateTime<Utc>,
    /// The public key of the user who made the payment
    pub sender: PublicKey,
    /// The amount of the payment
    pub amount: MicroTari,
    /// The memo attached to the transfer
    pub memo: Option<String>,
}
