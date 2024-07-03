use tpg_common::MicroTari;

use crate::ShopifyApiError;

/// Shopify uses floating point number expressed as strings.
pub fn parse_shopify_price(price: &str) -> Result<i64, ShopifyApiError> {
    let mut parts = price.split('.');
    let whole_units = parts
        .next()
        .ok_or_else(|| ShopifyApiError::InvalidCurrencyAmount(price.to_string()))?
        .parse::<i64>()
        .map_err(|e| ShopifyApiError::InvalidCurrencyAmount(format!("Invalid price value: {price}. {e}.")))?;
    let cents = parts
        .next()
        .map(|s| s.parse::<i64>())
        .unwrap_or(Ok(0))
        .map_err(|e| ShopifyApiError::InvalidCurrencyAmount(format!("Invalid price value: {price}. {e}.")))?;
    Ok(100 * whole_units + cents)
}

pub fn tari_shopify_price(p: MicroTari) -> String {
    format!("{}", p.value())
}
