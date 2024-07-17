use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ShopifyProduct {
    pub body_html: String,
    pub created_at: String,
    pub handle: String,
    pub id: i64,
    pub images: Option<Vec<ProductImage>>,
    pub product_type: String,
    pub published_at: String,
    pub published_scope: String,
    pub status: String,
    pub tags: String,
    pub template_suffix: Option<String>,
    pub title: String,
    pub updated_at: String,
    pub variants: Option<Vec<Variant>>,
    pub vendor: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Variant {
    pub barcode: Option<String>,
    pub compare_at_price: Option<String>,
    pub created_at: String,
    pub fulfillment_service: Option<String>,
    pub id: i64,
    pub inventory_item_id: i64,
    pub inventory_policy: String,
    pub inventory_quantity: i64,
    pub position: i64,
    pub price: String,
    pub product_id: i64,
    pub taxable: Option<bool>,
    pub title: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct ProductImage {
    pub id: i64,
    pub product_id: i64,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
    pub width: i64,
    pub height: i64,
    pub src: String,
    pub admin_graphql_api_id: Option<String>,
}
