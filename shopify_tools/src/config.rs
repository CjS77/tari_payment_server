use log::*;
use tpg_common::Secret;

#[derive(Debug, Clone, Default)]
pub struct ShopifyConfig {
    pub shop: String,
    pub admin_access_token: Secret<String>,
    pub api_version: String,
    pub shared_secret: Secret<String>,
}

impl ShopifyConfig {
    pub fn new_from_env_or_default() -> Self {
        let shop = std::env::var("TPG_SHOPIFY_SHOP").unwrap_or_else(|_| {
            warn!("TPG_SHOPIFY_SHOP not set, using (probably useless default");
            "example.myshopify.com".to_string()
        });
        let api_version = std::env::var("TPG_SHOPIFY_API_VERSION").unwrap_or_else(|_| {
            warn!("TPG_SHOPIFY_API_VERSION not set, using 2024-04 as default");
            "2024-04".to_string()
        });
        let admin_access_token = Secret::new(std::env::var("TPG_SHOPIFY_ADMIN_ACCESS_TOKEN").unwrap_or_else(|_| {
            warn!("TPG_SHOPIFY_ADMIN_ACCESS_TOKEN not set, using (probably useless default");
            "shpat_00000000000000".to_string()
        }));
        let shared_secret = Secret::new(std::env::var("TPG_SHOPIFY_API_SECRET").unwrap_or_else(|_| {
            warn!("TPG_SHOPIFY_SHARED_SECRET not set, using (probably useless default");
            "00000000000000".to_string()
        }));
        Self { shop, admin_access_token, api_version, shared_secret }
    }
}
