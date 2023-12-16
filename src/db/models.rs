use crate::db::errors::OrderConversionError;
use crate::spg_server::new_order::FreshOrder;
use chrono::{DateTime, Utc};
use sqlx::Type;
use std::fmt::Display;

#[derive(Debug, Clone, Type)]
#[sqlx(transparent)]
pub struct OrderId(String);

#[derive(Debug, Clone, Type)]
#[sqlx(transparent)]
pub struct MicroTari(i64);

impl From<i64> for MicroTari {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl PartialEq for MicroTari {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for MicroTari {}

impl TryFrom<u64> for MicroTari {
    type Error = OrderConversionError;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value > i64::MAX as u64 {
            Err(OrderConversionError(format!(
                "Value {} is too large to convert to MicroTari",
                value
            )))
        } else {
            #[allow(clippy::cast_possible_wrap)]
            Ok(Self(value as i64))
        }
    }
}

impl Display for MicroTari {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tari = self.0 as f64 / 1_000_000.0;
        write!(f, "{tari:0.3}τ")
    }
}

/// A lightweight wrapper around a string representing a public key
#[derive(Clone, Debug, Type)]
#[sqlx(transparent)]
pub struct PublicKey(pub String);

impl Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<S: Into<String>> From<S> for PublicKey {
    fn from(value: S) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone)]
pub enum OrderStatusType {
    /// The order has been paid but the payment is not confirmed.
    Pending,
    /// The order has been created and the payment has been received
    Confirmed,
    /// The order has been partially paid. The amount is the amount still owing
    PartiallyPaid(MicroTari),
    /// The order has been overpaid. The amount is the amount overpaid
    OverPaid(MicroTari),
    /// The order has been cancelled.
    Cancelled,
    /// The order has expired.
    Expired,
}

#[derive(Debug, Clone)]
pub struct OrderStatus {
    pub order_id: OrderId,
    pub updated_at: DateTime<Utc>,
    pub status: OrderStatusType,
}

pub struct Order {
    pub id: i64,
    pub order_id: String,
    pub customer_id: String,
    pub memo: Option<String>,
    pub total_price: MicroTari,
    pub currency: String,
    pub timestamp: DateTime<Utc>,
}

pub struct NewOrder {
    pub order_id: String,
    pub customer_id: String,
    pub memo: Option<String>,
    pub total_price: MicroTari,
    pub currency: String,
    pub timestamp: DateTime<Utc>,
}

impl NewOrder {
    pub fn is_equivalent(&self, order: &Order) -> bool {
        self.order_id == order.order_id
            && self.customer_id == order.customer_id
            && self.memo == order.memo
            && self.total_price == order.total_price
            && self.currency == order.currency
            && self.timestamp == order.timestamp
    }
}

impl TryFrom<FreshOrder> for NewOrder {
    type Error = OrderConversionError;
    fn try_from(value: FreshOrder) -> Result<Self, Self::Error> {
        let total_price = value
            .total_price
            .parse::<u64>()
            .map_err(|e| OrderConversionError(e.to_string()))
            .and_then(MicroTari::try_from)?;
        match value.currency.as_str() {
            "USD" | "XTR" => {}
            _ => {
                return Err(OrderConversionError(format!(
                    "Unsupported currency: {}",
                    value.currency
                )))
            }
        }
        let timestamp = value
            .created_at
            .parse::<DateTime<Utc>>()
            .map_err(|e| OrderConversionError(e.to_string()))?;
        Ok(Self {
            order_id: value.id.to_string(),
            customer_id: value.email,
            memo: value.note,
            total_price,
            currency: value.currency,
            timestamp,
        })
    }
}
