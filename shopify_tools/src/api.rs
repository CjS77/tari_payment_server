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

use crate::{config::ShopifyConfig, ExchangeRate, ExchangeRates, ShopifyApiError, ShopifyOrder};

pub struct ShopifyApi {
    config: ShopifyConfig,
    client: Arc<Client>,
}

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
        debug!("Sending REST query: {url}");
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
        trace!("GraphQL response: {data}");
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
        debug!("Fetched order #{order_id}");
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
        debug!("Cancelled order #{order_id}");
        Ok(result.order)
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
        trace!("Setting exchange rates: {variables}");
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
}
