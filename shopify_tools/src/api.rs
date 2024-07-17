use std::sync::Arc;

use graphql_parser::parse_query;
use log::*;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
    Method,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use tpg_common::MicroTari;

use crate::{
    config::ShopifyConfig,
    data_objects::{NewWebhook, ProductVariant, ProductVariants, Webhook},
    helpers::{parse_shopify_price, tari_shopify_price},
    ExchangeRate,
    ExchangeRates,
    ShopifyApiError,
    ShopifyOrder,
    ShopifyTransaction,
};

#[derive(Clone)]
pub struct ShopifyApi {
    config: ShopifyConfig,
    client: Arc<Client>,
}

const VARIANT_DEF: &str =
    "{ id product { id title } metafield(namespace: \"custom\" key: \"tari_price\") { id updatedAt value } price }";

impl ShopifyApi {
    pub fn new(config: ShopifyConfig) -> Result<Self, ShopifyApiError> {
        let mut headers = HeaderMap::with_capacity(2);
        let val = HeaderValue::from_str(config.admin_access_token.reveal().as_str())
            .map_err(|e| ShopifyApiError::Initialization(e.to_string()))?;
        headers.insert("X-Shopify-Access-Token", val);
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| ShopifyApiError::Initialization(e.to_string()))?;
        Ok(Self { config, client: Arc::new(client) })
    }

    pub async fn rest_query<T: DeserializeOwned, B: Serialize>(
        &self,
        method: Method,
        path: &str,
        params: &[(&str, &str)],
        body: Option<B>,
    ) -> Result<T, ShopifyApiError> {
        let url = self.url(path);
        trace!("Sending REST query: {url}");
        let mut req = self.client.request(method, url);
        if !params.is_empty() {
            req = req.query(params);
        }
        if let Some(body) = body {
            req = req.json(&body);
        }
        let response = req.send().await.map_err(|e| ShopifyApiError::RestResponseError(e.to_string()))?;
        if response.status().is_success() {
            trace!("REST query successful. {}", response.status());
            response.json::<T>().await.map_err(|e| ShopifyApiError::JsonError(e.to_string()))
        } else {
            let status = response.status().as_u16();
            let message = response.text().await.map_err(|e| ShopifyApiError::RestResponseError(e.to_string()))?;
            Err(ShopifyApiError::QueryError { status, message })
        }
    }

    pub async fn graphql_query<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: Option<Value>,
    ) -> Result<T, ShopifyApiError> {
        let query = parse_query::<String>(query).map_err(|e| ShopifyApiError::InvalidGraphQL(e.to_string()))?;
        let mut body = serde_json::json!({
            "query": query.to_string(),
        });
        if let Some(vars) = variables {
            body["variables"] = vars;
        }
        trace!("Sending GraphQL query: {body}");
        let result = self.rest_query::<Value, Value>(Method::POST, "/graphql.json", &[], Some(body)).await?;
        if let Some(errors) = result["errors"].as_array() {
            let e = errors.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(", ");
            return Err(ShopifyApiError::GraphQLError(e));
        }
        let data = result["data"].clone();
        let costs = result["extensions"]["cost"].clone();
        trace!("GraphQL response: {data}");
        trace!("GraphQL costs: {costs}");
        if data.is_null() {
            return Err(ShopifyApiError::EmptyResponse);
        }
        let result = serde_json::from_value(data).map_err(|e| ShopifyApiError::JsonError(e.to_string()))?;
        Ok(result)
    }

    pub fn url(&self, path: &str) -> String {
        format!("https://{}/admin/api/{}{path}", self.config.shop, self.config.api_version)
    }

    pub async fn get_order(&self, order_id: u64) -> Result<ShopifyOrder, ShopifyApiError> {
        #[derive(Deserialize)]
        struct OrderResponse {
            order: ShopifyOrder,
        }
        let path = format!("/orders/{order_id}.json");
        debug!("Fetching order #{order_id}");
        let result = self.rest_query::<OrderResponse, ()>(Method::GET, &path, &[], None).await?;
        info!("Fetched order #{order_id}");
        Ok(result.order)
    }

    pub async fn cancel_order(&self, order_id: u64) -> Result<ShopifyOrder, ShopifyApiError> {
        #[derive(Deserialize)]
        struct OrderResponse {
            order: ShopifyOrder,
        }
        let path = format!("/orders/{order_id}/cancel.json");
        debug!("Cancelling order #{order_id}");
        let result = self.rest_query::<OrderResponse, ()>(Method::POST, &path, &[], None).await?;
        info!("Cancelled order #{order_id}");
        Ok(result.order)
    }

    pub async fn mark_order_as_paid(
        &self,
        order_id: u64,
        amount: String,
        currency: String,
    ) -> Result<ShopifyTransaction, ShopifyApiError> {
        #[derive(Deserialize)]
        struct TransactionResponse {
            transaction: ShopifyTransaction,
        }
        let path = format!("/orders/{order_id}/transactions.json");
        let body = serde_json::json!({
            "transaction": {
                "parent_id": null,
                "amount": amount,
                "kind": "capture",
                "currency": currency,
            },
        });
        let result = self.rest_query::<TransactionResponse, Value>(Method::POST, &path, &[], Some(body)).await?;
        Ok(result.transaction)
    }

    pub async fn get_exchange_rates(&self) -> Result<ExchangeRates, ShopifyApiError> {
        let query = r#"{
          metaobjectByHandle(handle: {type: "tari_price", handle: "tari-price-global"}) {
            id handle type updatedAt fields { key value }
          }
        }"#;
        let value = self.graphql_query::<Value>(query, None).await?;
        let rates = ExchangeRates::from_metaobject(&value["metaobjectByHandle"])
            .map_err(|e| ShopifyApiError::JsonError(e.to_string()))?;
        Ok(rates)
    }

    pub async fn set_exchange_rates(&self, rates: &[ExchangeRate]) -> Result<ExchangeRates, ShopifyApiError> {
        let mutation = r#"
        mutation UpsertMetaobject($handle: MetaobjectHandleInput!, $metaobject: MetaobjectUpsertInput!) {
          metaobjectUpsert(handle: $handle, metaobject: $metaobject) {
            metaobject { id handle type updatedAt fields { key value } }
            userErrors { field message code }
          }
        }"#;
        let fields = rates
            .iter()
            .map(|r| serde_json::json!({"key": r.base_currency, "value": r.rate.value().to_string() }))
            .collect::<Vec<Value>>();
        let variables = serde_json::json!({
            "handle": { "type": "tari_price", "handle": "tari-price-global" },
            "metaobject": { "fields": fields }
        });
        debug!("Setting exchange rates: {variables}");
        let response = self.graphql_query::<Value>(mutation, Some(variables)).await?;
        if let Some(errors) = response["metaobjectUpsert"]["userErrors"].as_array() {
            if !errors.is_empty() {
                let e = errors.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(", ");
                return Err(ShopifyApiError::GraphQLError(e));
            }
        }
        let new_rates = &response["metaobjectUpsert"]["metaobject"];
        let new_rates =
            ExchangeRates::from_metaobject(new_rates).map_err(|e| ShopifyApiError::JsonError(e.to_string()))?;
        Ok(new_rates)
    }

    pub async fn fetch_variants(&self, after: Option<String>, count: u64) -> Result<ProductVariants, ShopifyApiError> {
        let after = after.map(|s| format!("\"{s}\"")).unwrap_or("null".to_string());
        let query = format!(
            "query {{productVariants(first: {count}, after: {after}) {{ pageInfo {{ endCursor hasNextPage }} nodes \
             {VARIANT_DEF} }}}}"
        );
        let result = self.graphql_query::<ProductVariants>(&query, None).await?;
        debug!(
            "Fetched {} variants. PageInfo: {} HasNextPage: {}",
            result.product_variants.nodes.len(),
            result.product_variants.page_info.end_cursor,
            result.product_variants.page_info.has_next_page
        );
        Ok(result)
    }

    pub async fn fetch_variant(&self, id: u64) -> Result<ProductVariant, ShopifyApiError> {
        #[derive(Deserialize)]
        struct ProductVariantResponse {
            #[serde(rename = "productVariant")]
            product_variant: Option<ProductVariant>,
        }
        let query = format!("query {{productVariant(id: \"gid://shopify/ProductVariant/{id}\") {VARIANT_DEF} }}");
        let result = self.graphql_query::<ProductVariantResponse>(&query, None).await?;
        let result = result.product_variant.ok_or(ShopifyApiError::EmptyResponse)?;
        debug!(
            "Fetched variant {id}: {} Price: {}. Tari price: {}",
            result.product.title,
            result.price,
            result.metafield.as_ref().map(|p| p.value.as_str()).unwrap_or("None")
        );
        Ok(result)
    }

    pub async fn fetch_all_variants(&self) -> Result<Vec<ProductVariant>, ShopifyApiError> {
        const FETCH_COUNT: u64 = 100;
        let mut variants = vec![];
        let mut after = None;
        loop {
            let result = self.fetch_variants(after, FETCH_COUNT).await?;
            let page_info = result.product_variants.page_info;
            variants.extend(result.product_variants.nodes);
            if !page_info.has_next_page {
                break;
            }
            after = Some(page_info.end_cursor);
        }
        Ok(variants)
    }

    /// Updates the price of the given product variants based on the given rate. Only those products that have in
    /// incorrect Tari price are updated.
    ///
    /// The list of variants is typically retrieved via a call to `fetch_all_variants`.
    ///
    /// The `metafield_id` is the ID of the metafield that contains the Tari price.
    /// The value of this id can be retrieved from the `ProductVariant` object.
    pub async fn update_tari_price(
        &self,
        products: &[ProductVariant],
        rate: ExchangeRate,
    ) -> Result<Vec<ProductVariant>, ShopifyApiError> {
        let mutation = format!(
            "mutation updateProductVariantMetafields($input: ProductVariantInput!) {{ productVariantUpdate(input: \
             $input) {{ productVariant {VARIANT_DEF} userErrors {{ message field }}  }} }}"
        );
        let rate = rate.rate.value();
        let mut result = vec![];
        debug!("Updating prices for {} product variants", products.len());
        for product in products {
            let shop_price_in_cents = parse_shopify_price(&product.price)?;
            let tari_price = tari_shopify_price(MicroTari::from(rate * shop_price_in_cents / 100));
            if let Some(mf) = &product.metafield {
                if mf.value == tari_price {
                    info!(
                        "Product variant {} ({}) has an up-to-date-price, so skipping its update",
                        product.id, product.product.title
                    );
                    continue;
                }
            }
            let metafield = match &product.metafield {
                Some(mf) => serde_json::json!({"id": mf.id,"value": tari_price}),
                None => {
                    serde_json::json!({"namespace": "custom","key": "tari_price","type": "number_integer","value": tari_price})
                },
            };
            let variables = serde_json::json!({ "input": {"id": product.id,"metafields": [metafield]}});
            debug!("Modifying product variant: {}", product.id);
            let response = self.graphql_query::<Value>(&mutation, Some(variables)).await?;
            if let Some(errors) = response["productVariantUpdate"]["userErrors"].as_array() {
                if !errors.is_empty() {
                    let e = errors.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(", ");
                    return Err(ShopifyApiError::GraphQLError(e));
                }
            }
            info!("Successfully updated price for {} ({})", product.id, product.product.title);
            let variant = serde_json::from_value(response["productVariantUpdate"]["productVariant"].clone())
                .map_err(|e| ShopifyApiError::JsonError(e.to_string()))?;
            result.push(variant);
        }
        Ok(result)
    }

    pub async fn update_all_prices(&self, rate: ExchangeRate) -> Result<Vec<ProductVariant>, ShopifyApiError> {
        let variants = self.fetch_all_variants().await?;
        self.update_tari_price(&variants, rate).await
    }

    pub async fn fetch_webhooks(&self) -> Result<Vec<Webhook>, ShopifyApiError> {
        #[derive(Deserialize)]
        struct WebhookResponse {
            webhooks: Vec<Webhook>,
        }
        debug!("Fetching webhooks");
        let result = self.rest_query::<WebhookResponse, ()>(Method::GET, "/webhooks.json", &[], None).await?;
        info!("Fetched webhooks");
        Ok(result.webhooks)
    }

    pub async fn install_webhook(&self, address: &str, topic: &str) -> Result<Webhook, ShopifyApiError> {
        #[derive(Serialize)]
        struct WebhookInput {
            webhook: NewWebhook,
        }
        #[derive(Deserialize)]
        struct WebhookResponse {
            webhook: Webhook,
        }
        let webhook = NewWebhook { topic: topic.to_string(), address: address.to_string(), format: "json".to_string() };
        let input = WebhookInput { webhook };
        debug!("Installing webhook: {}", serde_json::to_string(&input).unwrap_or_default());
        let result =
            self.rest_query::<WebhookResponse, WebhookInput>(Method::POST, "/webhooks.json", &[], Some(input)).await?;
        info!("Installed webhook: {:?}", result.webhook.id);
        Ok(result.webhook)
    }

    pub async fn update_webhook(&self, id: i64, new_address: &str) -> Result<Webhook, ShopifyApiError> {
        #[derive(Serialize)]
        struct UpdateWebhook {
            id: String,
            address: String,
        }
        #[derive(Serialize)]
        struct WebhookInput {
            webhook: UpdateWebhook,
        }
        #[derive(Deserialize)]
        struct WebhookResponse {
            webhook: Webhook,
        }
        let input = WebhookInput { webhook: UpdateWebhook { id: id.to_string(), address: new_address.to_string() } };
        let path = format!("/webhooks/{id}.json");
        debug!("Updating webhook: {}", serde_json::to_string(&input).unwrap_or_default());
        let result = self.rest_query::<WebhookResponse, WebhookInput>(Method::PUT, &path, &[], Some(input)).await?;
        info!("Updated webhook: {:?}", result.webhook.id);
        Ok(result.webhook)
    }
}
