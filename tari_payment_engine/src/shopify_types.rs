use chrono::{DateTime, Utc};
use shopify_tools::{CaptureTransaction, ShopifyPaymentCapture};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct ShopifyAuthorization {
    pub id: i64,
    pub order_id: i64,
    pub captured: bool,
    pub amount: String,
    pub currency: String,
    pub test: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ShopifyAuthorization> for ShopifyPaymentCapture {
    fn from(auth: ShopifyAuthorization) -> Self {
        let transaction = CaptureTransaction {
            parent_id: auth.id,
            kind: "capture".to_string(),
            amount: auth.amount,
            currency: auth.currency,
            test: auth.test,
        };
        Self { transaction }
    }
}

#[derive(Debug, Clone)]
pub struct NewShopifyAuthorization {
    pub id: i64,
    pub order_id: i64,
    pub captured: bool,
    pub amount: String,
    pub currency: String,
    pub test: bool,
}
