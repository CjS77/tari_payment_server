use crate::db::models::{MicroTari, OrderId};
use actix::Message;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
pub struct OrderUpdated {
    pub updated_at: DateTime<Utc>,
    pub order_id: OrderId,
    pub customer_id: String,
    pub memo: String, // This is used to match the order with the payment
    pub total_price: MicroTari,
    pub currency: String,
}
