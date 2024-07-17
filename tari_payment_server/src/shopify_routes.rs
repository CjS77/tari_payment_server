//----------------------------------------------   Checkout  ----------------------------------------------------

use actix_web::{web, HttpRequest, HttpResponse};
use log::{debug, error, info, trace, warn};
use shopify_tools::{
    data_objects::ExchangeRate as ShopifyExchangeRate,
    helpers::{parse_shopify_price, tari_shopify_price},
    ShopifyApi,
    ShopifyApiError,
    ShopifyOrder,
    ShopifyProduct,
};
use tari_payment_engine::{
    db_types::Role,
    tpe_api::{exchange_objects::ExchangeRate, exchange_rate_api::ExchangeRateApi},
    traits::{ExchangeRates, PaymentGatewayDatabase, PaymentGatewayError},
    OrderFlowApi,
};
use tpg_common::MicroTari;

use crate::{
    data_objects::{ExchangeRateUpdate, JsonResponse},
    errors::ServerError,
    integrations::shopify::{new_order_from_shopify_order, OrderConversionError},
    route,
};

route!(shopify_webhook => Post "webhook/checkout_create" impl PaymentGatewayDatabase, ExchangeRates);
pub async fn shopify_webhook<BPay, BFx>(
    req: HttpRequest,
    body: web::Json<ShopifyOrder>,
    api: web::Data<OrderFlowApi<BPay>>,
    fx: web::Data<ExchangeRateApi<BFx>>,
) -> HttpResponse
where
    BPay: PaymentGatewayDatabase,
    BFx: ExchangeRates,
{
    trace!("🛍️️ Received webhook request: {}", req.uri());
    let order = body.into_inner();
    // Webhook responses must always be in 200 range, otherwise Shopify will retry
    let result = match new_order_from_shopify_order(order, &fx).await {
        Err(OrderConversionError::FormatError(s)) => {
            warn!("🛍️️ Could not convert order. {s}");
            JsonResponse::failure(s)
        },
        Err(OrderConversionError::InvalidMemoSignature(e)) => {
            warn!("🛍️️ Could not verify memo signature. {e}");
            JsonResponse::failure(e)
        },
        Err(OrderConversionError::UnsupportedCurrency(cur)) => {
            info!("🛍️️ Unsupported currency in incoming order. {cur}");
            JsonResponse::failure(format!("Unsupported currency: {cur}"))
        },
        Ok(new_order) => match api.process_new_order(new_order.clone()).await {
            Ok(orders) => {
                info!("🛍️️ Order {} processed successfully.", new_order.order_id);
                let ids = orders.iter().map(|o| o.order_id.as_str()).collect::<Vec<_>>().join(", ");
                info!("🛍️️ {} orders were paid. {}", orders.len(), ids);
                JsonResponse::success("Order processed successfully.")
            },
            Err(PaymentGatewayError::DatabaseError(e)) => {
                warn!("🛍️️ Could not process order {}. {e}", new_order.order_id);
                debug!("🛍️️ Failed order: {new_order}");
                JsonResponse::failure(e)
            },
            Err(PaymentGatewayError::OrderAlreadyExists(id)) => {
                info!("🛍️️ Order {} already exists with id {id}.", new_order.order_id);
                JsonResponse::success("Order already exists.")
            },
            Err(e) => {
                warn!("🛍️️ Unexpected error while handling incoming order notification. {e}");
                JsonResponse::failure("Unexpected error handling order.")
            },
        },
    };
    HttpResponse::Ok().json(result)
}

route!(shopify_on_product_updated => Post "webhook/product_updated" impl ExchangeRates);
pub async fn shopify_on_product_updated<BFx>(
    body: web::Json<ShopifyProduct>,
    shopify_api: web::Data<ShopifyApi>,
    fx: web::Data<ExchangeRateApi<BFx>>,
) -> HttpResponse
where
    BFx: ExchangeRates,
{
    let product = body.into_inner();
    let current_rate = match fx.fetch_last_rate("USD").await {
        Ok(cr) => cr,
        Err(e) => {
            error!("🛍️️  Could not fetch exchange rate. {e}");
            // Shopify expects a 200 response
            return HttpResponse::Ok().finish();
        },
    };
    debug!(
        "🛍️️  Received shopify product update webhook call for product {} ({}). Checking product variants",
        product.title, product.id
    );
    if let Some(variants) = product.variants.as_ref() {
        let mut variants_to_update = vec![];
        for variant in variants {
            match shopify_api.fetch_variant(variant.id).await {
                Ok(v) => {
                    let shop_price_in_cents = match parse_shopify_price(&v.price) {
                        Ok(v) => v,
                        Err(e) => {
                            warn!("🛍️️ Could not parse price for variant {}. {e}", variant.id);
                            continue;
                        },
                    };
                    let expected_price =
                        tari_shopify_price(MicroTari::from(current_rate.rate.value() * shop_price_in_cents / 100));
                    let needs_update = v.metafield.as_ref().map(|m| m.value != expected_price).unwrap_or(true);
                    if needs_update {
                        warn!("🛍️️  Variant {} price is out of date. Queing it for updating.", variant.id);
                        variants_to_update.push(v);
                    } else {
                        debug!("🛍️️  Variant {} price is up to date. No further action to take", variant.id);
                    }
                },
                Err(ShopifyApiError::EmptyResponse) => {
                    warn!(
                        "🛍️️ Variant {} not found for product {}({}). The product might just have been deleted, or this \
                         could be a bug",
                        variant.id, product.title, product.id
                    );
                },
                Err(e) => {
                    error!("🛍️️ Error checking product variant {} price. {e}", variant.id);
                },
            }
        }
        if !variants_to_update.is_empty() {
            debug!("🛍️️  Updating prices for {} variants", variants_to_update.len());
            let rate = ShopifyExchangeRate::new("USD".to_string(), current_rate.rate);
            shopify_api.update_tari_price(&variants_to_update, rate).await.map(|_| ()).unwrap_or_else(|e| {
                error!("🛍️️ Could not update variant prices on Shopify. {e}");
            });
        }
    }
    HttpResponse::Ok().finish()
}

route!(update_shopify_exchange_rate => Post "/exchange_rate" impl ExchangeRates where requires [Role::Write]);
pub async fn update_shopify_exchange_rate<B: ExchangeRates>(
    body: web::Json<ExchangeRateUpdate>,
    api: web::Data<ExchangeRateApi<B>>,
    shopify_api: web::Data<ShopifyApi>,
) -> Result<HttpResponse, ServerError> {
    let update = body.into_inner();
    debug!("🛍️️  POST update exchange rate for {} to {}", update.currency, MicroTari::from(update.rate as i64));
    update_local_exchange_rate(update.clone(), api.as_ref()).await?;
    debug!("🛍️️  Tari price has been updated in the database.");
    update_shopify_exchange_rate_for(&update, shopify_api.as_ref()).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn update_local_exchange_rate<B: ExchangeRates>(
    update: ExchangeRateUpdate,
    api: &ExchangeRateApi<B>,
) -> Result<(), ServerError> {
    let rate = ExchangeRate::from(update);
    debug!("🛍️️  POST update exchange rate for {rate}");
    api.set_exchange_rate(&rate).await.map_err(|e| {
        debug!("🛍️️  Could not update exchange rate. {e}");
        ServerError::BackendError(e.to_string())
    })
}

async fn update_shopify_exchange_rate_for(
    update: &ExchangeRateUpdate,
    shopify_api: &ShopifyApi,
) -> Result<(), ServerError> {
    #[allow(clippy::cast_possible_wrap)]
    let rate = ShopifyExchangeRate::new(update.currency.to_string(), MicroTari::from(update.rate as i64));
    debug!("🛍️️ Updating prices on Shopify storefront 1 {} = {}", rate.base_currency, rate.rate);
    match shopify_api.update_all_prices(rate).await {
        Ok(v) => {
            info!("🛍️️ {} variant prices updated on shopify storefront.", v.len());
            Ok(())
        },
        Err(e) => {
            error!("🛍️️ Could not update variant prices on Shopify. {e}");
            Err(ServerError::BackendError(e.to_string()))
        },
    }
}
