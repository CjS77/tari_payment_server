use crate::db_types::OrderStatus;

use crate::db_types::{MicroTari, PublicKey};
use chrono::{DateTime, Utc};

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
