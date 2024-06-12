use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db_types::{MicroTari, Order, OrderStatus, OrderStatusType, PublicKey};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderAnnulledEvent {
    pub order: Order,
    pub status: OrderStatusType,
}

impl OrderAnnulledEvent {
    pub fn new(order: Order) -> Self {
        let status = order.status;
        Self { order, status }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderModifiedEvent {
    pub old_order: Order,
    pub new_order: Order,
}

impl OrderModifiedEvent {
    pub fn new(old_order: Order, new_order: Order) -> Self {
        Self { old_order, new_order }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    OrderPaid(OrderPaidEvent),
    OrderAnnulled(OrderAnnulledEvent),
    OrderModified(OrderModifiedEvent),
}
