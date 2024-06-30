use chrono::{DateTime, Utc};
use log::{info, trace};
use shopify_tools::ShopifyOrder;
use tari_payment_engine::{
    db_types::{NewOrder, OrderId},
    helpers::MemoSignatureError,
    tpe_api::{exchange_objects::ExchangeRate, exchange_rate_api::ExchangeRateApi},
    traits::ExchangeRates,
};
use thiserror::Error;
use tpg_common::TARI_CURRENCY_CODE;

#[derive(Debug, Error)]
#[error("Could not convert shopify order into a new order. {0}.")]
pub enum OrderConversionError {
    #[error("The Shopify order contained invalid data. {0}")]
    FormatError(String),
    #[error("{0} is not a supported currency.")]
    UnsupportedCurrency(String),
    #[error("The memo signature was invalid. {0}")]
    InvalidMemoSignature(#[from] MemoSignatureError),
}

pub async fn new_order_from_shopify_order<B: ExchangeRates>(
    value: ShopifyOrder,
    fx: &ExchangeRateApi<B>,
) -> Result<NewOrder, OrderConversionError> {
    trace!("Converting ShopifyOrder to NewOrder: {:?}", value);
    let currency = value.currency.as_str().to_uppercase();
    let rate = if currency != TARI_CURRENCY_CODE {
        let rate = fx
            .fetch_last_rate(&currency)
            .await
            .map_err(|e| OrderConversionError::UnsupportedCurrency(e.to_string()))?;
        info!("Shopify order is not in Tari. Using a conversion rate of {rate}");
        rate
    } else {
        ExchangeRate::default()
    };
    // Net price in cents.
    let total_price = parse_shopify_price(&value.total_price)?;
    let total_price = rate.convert_to_tari_from_cents(total_price);
    let timestamp =
        value.created_at.parse::<DateTime<Utc>>().map_err(|e| OrderConversionError::FormatError(e.to_string()))?;
    let memo = value.note;
    let mut order = NewOrder {
        order_id: OrderId(value.id.to_string()),
        customer_id: value.customer.id.to_string(),
        currency: value.currency,
        memo,
        address: None,
        created_at: timestamp,
        total_price,
    };
    if let Err(e) = order.try_extract_address() {
        info!(
            "Order {} did not contain a valid signature. This order is going to remain unclaimed. Error: {e}. Memo: {}",
            order.order_id,
            order.memo.as_ref().unwrap_or(&"No memo provided".to_string())
        );
    }
    Ok(order)
}

/// Shopify uses floating point number expressed as strings.
fn parse_shopify_price(price: &str) -> Result<i64, OrderConversionError> {
    let mut parts = price.split('.');
    let whole_units = parts
        .next()
        .ok_or_else(|| OrderConversionError::FormatError(format!("Empty price in shopify order: {price}")))?
        .parse::<i64>()
        .map_err(|e| OrderConversionError::FormatError(e.to_string()))?;
    let cents = parts
        .next()
        .map(|s| s.parse::<i64>())
        .unwrap_or(Ok(0))
        .map_err(|e| OrderConversionError::FormatError(e.to_string()))?;
    Ok(100 * whole_units + cents)
}
