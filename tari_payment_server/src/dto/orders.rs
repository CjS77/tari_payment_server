use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopifyOrder {
    /// The order_id as assigned by Shopify
    pub order_id: String,
    /// The customer_id as assigned by Shopify
    pub customer_id: String,
    /// An optional description supplied by the user for the order. Useful for matching orders with payments
    pub memo: Option<String>,
    /// The total price of the order
    pub total_price: i64,
    /// The currency of the order
    pub currency: String,
    /// The time the order was created on Shopify
    pub created_at: DateTime<Utc>,
}
