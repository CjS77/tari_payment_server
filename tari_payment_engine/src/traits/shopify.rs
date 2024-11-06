use thiserror::Error;

use crate::shopify_types::{NewShopifyAuthorization, ShopifyAuthorization};

#[derive(Debug, Clone, Error)]
pub enum ShopifyAuthorizationError {
    #[error("Shopify Authorization {0} for Order {1} not found")]
    NotFound(i64, i64),
    #[error("Shopify Authorization {0} for Order {1} already exists")]
    AlreadyExists(i64, i64),
    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl From<sqlx::Error> for ShopifyAuthorizationError {
    fn from(e: sqlx::Error) -> Self {
        ShopifyAuthorizationError::DatabaseError(e.to_string())
    }
}

#[allow(async_fn_in_trait)]
pub trait ShopifyAuthorizations {
    async fn insert_new(
        &self,
        auth: NewShopifyAuthorization,
    ) -> Result<ShopifyAuthorization, ShopifyAuthorizationError>;
    /// Fetch all authorizations for the given order id.
    async fn fetch_by_order_id(&self, order_id: i64) -> Result<Vec<ShopifyAuthorization>, ShopifyAuthorizationError>;
    /// Set all authorizations for the given order id to the given status.
    /// Returns the updated transaction records.
    async fn capture(&self, order_id: i64, capture: bool) -> Result<Vec<ShopifyAuthorization>, ShopifyAuthorizationError>;
}
