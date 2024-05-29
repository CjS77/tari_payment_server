use chrono::{DateTime, Utc};

use crate::db_types::{MicroTari, Order, OrderStatus, PublicKey};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderPaidEvent {
    pub order: Order,
}

impl OrderPaidEvent {
    pub fn new(order: Order) -> Self {
        Self { order }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    OrderPaid(OrderPaidEvent),
}