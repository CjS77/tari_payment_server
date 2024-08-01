use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use log::*;
use shopify_tools::{
    helpers::parse_shopify_price,
    ShopifyApi,
    ShopifyApiError,
    ShopifyConfig as ShopifyApiConfig,
    ShopifyOrder,
};
use tari_payment_engine::{
    db_types::{NewOrder, Order, OrderId},
    events::{EventHandlers, EventHooks, OrderAnnulledEvent},
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
    let rate = if currency == TARI_CURRENCY_CODE {
        ExchangeRate::default()
    } else {
        let rate = fx
            .fetch_last_rate(&currency)
            .await
            .map_err(|e| OrderConversionError::UnsupportedCurrency(e.to_string()))?;
        info!("Shopify order is not in Tari. Using a conversion rate of {rate}");
        rate
    };
    // Net price in cents.
    let total_price =
        parse_shopify_price(&value.total_price).map_err(|e| OrderConversionError::FormatError(e.to_string()))?;
    let total_price = rate.convert_to_tari_from_cents(total_price);
    let timestamp =
        value.created_at.parse::<DateTime<Utc>>().map_err(|e| OrderConversionError::FormatError(e.to_string()))?;
    let memo = value.note;
    let mut order = NewOrder {
        order_id: OrderId(value.id.to_string()),
        customer_id: value.customer.id.to_string(),
        currency: value.currency,
        original_price: Some(value.total_price),
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

pub const SHOPIFY_EVENT_BUFFER_SIZE: usize = 25;

/// Assigns event handlers to the Shopify API.
///
/// Only the following events are relevant to interacting with the Shopify API:
///
/// 1. OrderPaidEvent - Once an order is marked as paid in the payment engine, we send a REST request to the Shopify API
///    to mark the order as fulfilled.
/// 2. OrderAnnulledEvent - If an order is cancelled or expires, we send a REST request to the Shopify API to mark the
///    order as cancelled. If an order is expired from the Shopify Admin UI, then this REST call will be spurious, but
///    no harm will be done.
pub fn create_shopify_event_handlers(config: ShopifyApiConfig) -> Result<EventHandlers, ShopifyApiError> {
    let mut hooks = EventHooks::default();
    let api = ShopifyApi::new(config)?;
    let api_clone = api.clone();
    // --- On OrderPaid Handler ---
    hooks.on_order_paid(move |ev| {
        let order = ev.order;
        let order_id = match parse_shopify_order_id(&order) {
            Some(value) => value,
            None => return no_op(),
        };
        let Some(original_price) = order.original_price else {
            error!(
                "🛍️ The order that has just been marked as paid does not have an original price. Shopify orders \
                 should
            have populated this field. TODO: Calculate the original price from the prevailing Tari price. Order \
                 details: {order:?}"
            );
            return no_op();
        };
        let api_clone = api_clone.clone();
        Box::pin(async move {
            match api_clone.mark_order_as_paid(order_id, original_price, order.currency).await {
                Ok(tx) => info!(
                    "🛍️ Order {order_id} marked as paid on Shopify. New status: {}. Tx id: {}. Errors (if any): {} {}",
                    tx.status,
                    tx.id,
                    tx.error_code.unwrap_or_else(|| "None".to_string()),
                    tx.message
                ),
                Err(e) => error!("🛍️ Error marking order {order_id} as paid on Shopify. {e}"),
            }
        })
    });
    // --- On OrderAnnulled Handler ---
    hooks.on_order_annulled(move |ev| {
        let OrderAnnulledEvent { order, status } = ev;
        let order_id = match parse_shopify_order_id(&order) {
            Some(value) => value,
            None => return no_op(),
        };
        let api_clone = api.clone();
        debug!("🛍️ Order {order_id} has been annulled. Reason: {status}. Sending cancellation request to Shopify.");
        Box::pin(async move {
            match api_clone.cancel_order(order_id).await {
                Ok(o) => info!(
                    "🛍️ Order {order_id} has been cancelled on Shopify. Reason: {}. Timestamp: {}",
                    o.cancel_reason.unwrap_or_default(),
                    o.cancelled_at.unwrap_or_default()
                ),
                Err(e) => error!("🛍️ Error marking order {order_id} as paid on Shopify. {e}"),
            }
        })
    });
    let handlers = EventHandlers::new(SHOPIFY_EVENT_BUFFER_SIZE, hooks);
    Ok(handlers)
}

fn parse_shopify_order_id(order: &Order) -> Option<u64> {
    match order.order_id.as_str().parse::<u64>() {
        Ok(v) => Some(v),
        Err(e) => {
            error!(
                "🛍️ Shopify order ids must be integers. An order that has just been marked as paid could not be \
                 converted into a Shopify Order id. Error: {e}. Order details: {order:?}"
            );
            None
        },
    }
}

fn no_op() -> BoxFuture<'static, ()> {
    Box::pin(async {})
}
