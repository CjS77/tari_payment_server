use std::fmt::Debug;

use log::*;

use crate::{
    shopify_types::{NewShopifyAuthorization, ShopifyAuthorization},
    traits::{ShopifyAuthorizationError, ShopifyAuthorizations},
};

pub struct ShopifyTrackerApi<B> {
    db: B,
}

impl<B> Debug for ShopifyTrackerApi<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ShopifyTrackerApi")
    }
}

impl<B> Clone for ShopifyTrackerApi<B>
where B: ShopifyAuthorizations + Clone
{
    fn clone(&self) -> Self {
        Self { db: self.db.clone() }
    }
}

impl<B> ShopifyTrackerApi<B>
where B: ShopifyAuthorizations
{
    pub fn new(db: B) -> Self {
        Self { db }
    }

    pub async fn log_authorization(&self, auth: NewShopifyAuthorization) -> Result<(), ShopifyAuthorizationError> {
        let desc = format!("tx {} (Order {})", auth.id, auth.order_id);
        info!("📋️☑️ Logging new shopify authorization for {desc}");
        match self.db.insert_new(auth).await {
            Ok(_) => {
                info!("📋️☑️ Shopify order {desc} added to tracking");
                Ok(())
            },
            Err(ShopifyAuthorizationError::AlreadyExists(_, _)) => {
                info!("📋️☑️ Shopify order {desc} already exists in tracking");
                Ok(())
            },
            Err(e) => {
                info!("📋️☑️ Shopify order {desc} failed to be added to tracking: {e}");
                Err(e)
            },
        }
    }

    pub async fn fetch_payment_auth(
        &self,
        order_id: i64,
    ) -> Result<Vec<ShopifyAuthorization>, ShopifyAuthorizationError> {
        trace!("📋️☑️ Fetching tracking order: {order_id}");
        self.db.fetch_by_order_id(order_id).await
    }

    pub async fn set_capture_flag(&self, order_id: i64, capture_flag: bool) -> Result<(), ShopifyAuthorizationError> {
        trace!("📋️☑️ Setting capture flag for order {order_id} to {capture_flag}");
        let _ = self.db.capture(order_id, capture_flag).await?;
        Ok(())
    }
}
