use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tpg_common::MicroTari;

use crate::ShopifyApiError;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ExchangeRates {
    id: String,
    updated_at: DateTime<Utc>,
    rates: Vec<ExchangeRate>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ExchangeRate {
    pub base_currency: String,
    pub rate: MicroTari,
}

impl ExchangeRate {
    pub fn new(base_currency: String, rate: MicroTari) -> Self {
        Self { base_currency, rate }
    }

    pub fn from_metaobject_field(field: &Value) -> Result<Self, ShopifyApiError> {
        let base_currency = field["key"]
            .as_str()
            .ok_or(ShopifyApiError::JsonError("'key' does not exist in field".to_string()))?
            .to_string();
        let rate = field["value"]
            .as_str()
            .ok_or(ShopifyApiError::JsonError("'value' does not exist in field".to_string()))?
            .to_string()
            .parse::<u64>()
            .map_err(|e| ShopifyApiError::JsonError(format!("Invalid exchange rate value. {e}")))?;
        #[allow(clippy::cast_possible_wrap)]
        Ok(Self::new(base_currency, MicroTari::from(rate as i64)))
    }
}

impl ExchangeRates {
    pub fn from_metaobject(metaobject: &Value) -> Result<Self, ShopifyApiError> {
        match metaobject["type"].as_str() {
            None => {
                return Err(ShopifyApiError::JsonError("Not an ExchangeRate MetaObject. Missing 'type'".to_string()))
            },
            Some(t) if t != "tari_price" => {
                return Err(ShopifyApiError::JsonError(format!(
                    "Not an ExchangeRate MetaObject. 'type' should be 'tari_price', not '{t}'"
                )))
            },
            _ => (),
        }
        match metaobject["handle"].as_str() {
            None => {
                return Err(ShopifyApiError::JsonError("Not an ExchangeRate MetaObject. Missing 'handle'".to_string()))
            },
            Some(t) if t != "tari-price-global" => {
                return Err(ShopifyApiError::JsonError(format!("Expecting 'tari-price-global', not '{t}'")))
            },
            _ => (),
        }
        let updated_at = metaobject["updatedAt"]
            .as_str()
            .ok_or(ShopifyApiError::JsonError("'updated_at' does not exist in metaobject".to_string()))?
            .parse::<DateTime<Utc>>()
            .map_err(|e| ShopifyApiError::JsonError(format!("Invalid updated_at value. {e}")))?;
        let id = metaobject["id"]
            .as_str()
            .ok_or_else(|| ShopifyApiError::JsonError("'id' does not exist in metaobject".to_string()))?
            .to_string();
        let rates = metaobject["fields"]
            .as_array()
            .map(|fields| {
                fields
                    .iter()
                    .map(ExchangeRate::from_metaobject_field)
                    .collect::<Result<Vec<ExchangeRate>, ShopifyApiError>>()
            })
            .unwrap_or_else(|| Ok(vec![]))?;
        Ok(Self { id, updated_at, rates })
    }
}
