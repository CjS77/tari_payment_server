use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use log::*;
use shopify_tools::{
    helpers::parse_shopify_price,
    ShopifyApi,
    ShopifyApiError,
    ShopifyConfig as ShopifyApiConfig,
    ShopifyOrder,
    ShopifyPaymentCapture,
    ShopifyTransaction,
};
use tari_payment_engine::{
    db_types::{NewOrder, Order, OrderId},
    events::{EventHandlers, EventHooks, OrderAnnulledEvent},
    helpers::MemoSignatureError,
    shopify_types::NewShopifyAuthorization,
    tpe_api::{
        exchange_objects::ExchangeRate,
        exchange_rate_api::ExchangeRateApi,
        shopify_tracker_api::ShopifyTrackerApi,
    },
    traits::ExchangeRates,
    SqliteDatabase,
};
use thiserror::Error;
use tpg_common::TARI_CURRENCY_CODE;

use crate::config::ShopifyPriceField;

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
    price_field: ShopifyPriceField,
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
    debug!(
        "Order {}({}) price information. Total: {}, Line Items: {}, Subtotal: {}",
        value.id, value.name, value.total_price, value.total_line_items_price, value.subtotal_price
    );
    debug!("Using price field: {price_field}");
    let price_field = match price_field {
        ShopifyPriceField::TotalPrice => value.total_price,
        ShopifyPriceField::LineItemsPrice => value.total_line_items_price,
        ShopifyPriceField::SubtotalPrice => value.subtotal_price,
    };
    // Net price in cents.
    let total_price =
        parse_shopify_price(&price_field).map_err(|e| OrderConversionError::FormatError(e.to_string()))?;
    let total_price = rate.convert_to_tari_from_cents(total_price);
    trace!("Interpreting order price as: {total_price}");
    let timestamp =
        value.created_at.parse::<DateTime<Utc>>().map_err(|e| OrderConversionError::FormatError(e.to_string()))?;
    let memo = value.note;
    let mut order = NewOrder {
        order_id: OrderId::from(value.id),
        alt_order_id: Some(OrderId::from(value.name)),
        customer_id: value.customer.id.to_string(),
        currency: value.currency,
        original_price: Some(price_field),
        memo,
        address: None,
        created_at: timestamp,
        total_price,
        amount_outstanding: Some(value.total_outstanding),
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
pub fn create_shopify_event_handlers(
    config: ShopifyApiConfig,
    tracker: ShopifyTrackerApi<SqliteDatabase>,
) -> Result<EventHandlers, ShopifyApiError> {
    let mut hooks = EventHooks::default();
    let must_capture_payment = config.capture_payments;
    let api = ShopifyApi::new(config)?;
    let api_clone = api.clone();
    let tracker_clone = tracker.clone();
    // --- On OrderPaid Handler ---
    hooks.on_order_paid(move |ev| {
        let order = ev.order;
        let order_id = match parse_shopify_order_id(&order) {
            Some(value) => value,
            None => return no_op(),
        };
        let amount_to_pay = match (must_capture_payment, order.amount_outstanding.clone(), order.original_price.clone())
        {
            (false, _, Some(p)) => p,
            (true, Some(p), _) => p,
            (false, Some(p), None) => {
                warn!(
                    "🛍️ The order that has just been marked as paid does not have an original price. Used the \
                     outstanding amount instead. {order:?}"
                );
                p
            },
            (true, None, Some(p)) => {
                warn!(
                    "🛍️ The order that has just been marked as paid does not have an outstanding amount, but we are \
                     being asked to capture external payments. It's possible that this payment request will fail and \
                     will require a manual override in the storefront. {order:?}"
                );
                p
            },
            (_, None, None) => {
                error!(
                    "🛍️ The order that has just been marked as paid does not have an original or an outstanding \
                     amount. A manual override in the storefront is required. {order:?}"
                );
                return no_op();
            },
        };
        let api_clone = api_clone.clone();
        let tracker_clone = tracker_clone.clone();
        Box::pin(async move {
            if must_capture_payment {
                let oid = order_id as i64;
                let auths = tracker_clone.fetch_payment_auth(oid).await.unwrap_or_else(|e| {
                    error!(
                        "🛍️ Error fetching payment authorizations for order {order_id} from the database. {e}. Manual \
                         intervention is required."
                    );
                    vec![]
                });
                for auth in auths {
                    if !auth.captured {
                        let capture = ShopifyPaymentCapture::from(auth);
                        match api_clone.capture_payment(oid as i64, capture).await {
                            Ok(t) => {
                                info!(
                                    "🛍️ Order {order_id} payment captured on Shopify. Tx: {} Order: {}. Kind: {}. {}",
                                    t.id, t.order_id, t.kind, t.message
                                );
                            },
                            Err(e) => {
                                error!(
                                    "🛍️ Error capturing payment for order {order_id} on Shopify. Manual intervention \
                                     is required. {e}"
                                );
                            },
                        }
                    }
                }
            }
            let due = parse_shopify_price(&amount_to_pay).unwrap_or(1);
            if due == 0 {
                info!(
                    "🛍️ Order {order_id} has been marked as paid on the server, but the amount due is 0. No further \
                     action required on the storefront."
                );
                return;
            }
            match api_clone.mark_order_as_paid(order_id, amount_to_pay, order.currency).await {
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

pub fn shopify_auth_from_tx(tx: &ShopifyTransaction) -> NewShopifyAuthorization {
    NewShopifyAuthorization {
        id: tx.id,
        order_id: tx.order_id,
        amount: tx.amount.clone(),
        currency: tx.currency.clone(),
        test: tx.test,
        captured: false,
    }
}
