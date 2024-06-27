use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tari_common_types::tari_address::TariAddress;
use tpg_common::MicroTari;

use crate::{
    db_types::{Order, OrderStatus, OrderStatusType, Payment, PublicKey},
    order_objects::OrderChanged,
};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderEvent {
    pub order: Order,
}

impl OrderEvent {
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
    pub field_changed: String,
    pub orders: OrderChanged,
}

impl OrderModifiedEvent {
    pub fn new(field_changed: String, orders: OrderChanged) -> Self {
        Self { field_changed, orders }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderClaimedEvent {
    pub order: Order,
    pub claimant: TariAddress,
}

impl OrderClaimedEvent {
    pub fn new(order: Order, claimant: TariAddress) -> Self {
        Self { order, claimant }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentEvent {
    pub payment: Payment,
}

impl PaymentEvent {
    pub fn new(payment: Payment) -> Self {
        Self { payment }
    }
}

impl From<Payment> for PaymentEvent {
    fn from(payment: Payment) -> Self {
        Self::new(payment)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    NewOrder(OrderEvent),
    OrderPaid(OrderEvent),
    OrderAnnulled(OrderAnnulledEvent),
    OrderModified(OrderModifiedEvent),
    PaymentReceived(PaymentEvent),
    Confirmation(PaymentEvent),
}
