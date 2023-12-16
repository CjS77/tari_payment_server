use crate::db::models::OrderStatus;
use actix::Message;

#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
pub struct OrderStatusMessage(pub OrderStatus);
