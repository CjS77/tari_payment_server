use chrono::{DateTime, Utc};
use log::{info, trace};
use shopify_tools::ShopifyOrder;
use tari_payment_engine::{
    db_types::{NewOrder, OrderId},
    helpers::MemoSignatureError,
};
use thiserror::Error;
use tpg_common::{MicroTari, TARI_CURRENCY_CODE_LOWER};

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

pub fn new_order_from_shopify_order(value: ShopifyOrder) -> Result<NewOrder, OrderConversionError> {
    trace!("Converting ShopifyOrder to NewOrder: {:?}", value);
    if value.currency.as_str().to_lowercase() != TARI_CURRENCY_CODE_LOWER {
        return Err(OrderConversionError::UnsupportedCurrency(value.currency));
    }
    let total_price = value
        .total_price
        .parse::<u64>()
        .map_err(|e| OrderConversionError::FormatError(e.to_string()))
        .map(MicroTari::try_from)?
        .map_err(|e| OrderConversionError::FormatError(e.to_string()))?;

    let timestamp =
        value.created_at.parse::<DateTime<Utc>>().map_err(|e| OrderConversionError::FormatError(e.to_string()))?;
    let memo = value.note;
    let mut order = NewOrder {
        order_id: OrderId(value.name),
        customer_id: value.customer.id.to_string(),
        currency: value.currency,
        memo,
        address: None,
        created_at: timestamp,
        total_price,
    };
    if let Err(e) = order.try_extract_address() {
        info!("Order {} did not contain a signed memo. This order is going to remain unclaimed. Error: {e}", order.order_id);
    }
    Ok(order)
}
