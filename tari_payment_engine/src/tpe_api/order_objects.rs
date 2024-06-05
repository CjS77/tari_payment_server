use std::{fmt::Display, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tari_common_types::tari_address::TariAddress;

use crate::{
    db_types::{MicroTari, Order, OrderId, OrderStatusType},
    traits::AccountApiError,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderResult {
    #[serde(serialize_with = "address_to_hex")]
    pub address: TariAddress,
    pub total_orders: MicroTari,
    pub orders: Vec<Order>,
}

pub fn address_to_hex<S>(address: &TariAddress, serializer: S) -> Result<S::Ok, S::Error>
where S: serde::Serializer {
    serializer.serialize_str(&address.to_hex())
}

pub fn str_to_address<'de, D>(deserializer: D) -> Result<TariAddress, D::Error>
where D: serde::Deserializer<'de> {
    let s = String::deserialize(deserializer)?;
    TariAddress::from_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderQueryFilter {
    pub memo: Option<String>,
    pub order_id: Option<OrderId>,
    pub account_id: Option<i64>,
    pub customer_id: Option<String>,
    pub currency: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub status: Option<Vec<OrderStatusType>>,
}

impl OrderQueryFilter {
    pub fn with_memo(mut self, memo: String) -> Self {
        self.memo = Some(memo);
        self
    }

    pub fn since<T>(mut self, since: T) -> Result<Self, AccountApiError>
    where
        T: TryInto<DateTime<Utc>>,
        T::Error: Display,
    {
        let dt = since.try_into().map_err(|e| AccountApiError::QueryError(e.to_string()))?;
        self.since = Some(dt);
        Ok(self)
    }

    pub fn until<T>(mut self, until: T) -> Result<Self, AccountApiError>
    where
        T: TryInto<DateTime<Utc>>,
        T::Error: Display,
    {
        let dt = until.try_into().map_err(|e| AccountApiError::QueryError(e.to_string()))?;
        self.until = Some(dt);
        Ok(self)
    }

    pub fn with_account_id(mut self, account_id: i64) -> Self {
        self.account_id = Some(account_id);
        self
    }

    pub fn with_order_id(mut self, order_id: OrderId) -> Self {
        self.order_id = Some(order_id);
        self
    }

    pub fn with_customer_id(mut self, customer_id: String) -> Self {
        self.customer_id = Some(customer_id);
        self
    }

    pub fn with_currency(mut self, currency: String) -> Self {
        self.currency = Some(currency);
        self
    }

    pub fn with_status(mut self, status: OrderStatusType) -> Self {
        if self.status.is_none() {
            self.status = Some(vec![status]);
        } else {
            self.status.as_mut().unwrap().push(status);
        }

        self
    }

    pub fn is_empty(&self) -> bool {
        self.memo.is_none() &&
            self.order_id.is_none() &&
            self.account_id.is_none() &&
            self.customer_id.is_none() &&
            self.currency.is_none() &&
            self.status.is_none() &&
            self.since.is_none() &&
            self.until.is_none()
    }
}

impl Display for OrderQueryFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            write!(f, "No filters.")?;
            return Ok(());
        }
        if let Some(memo) = &self.memo {
            write!(f, "memo: {memo}. ")?;
        }
        if let Some(order_id) = &self.order_id {
            write!(f, "order_id: {order_id}. ")?;
        }
        if let Some(account_id) = &self.account_id {
            write!(f, "account_id: {account_id}. ")?;
        }
        if let Some(customer_id) = &self.customer_id {
            write!(f, "customer_id: {customer_id}. ")?;
        }
        if let Some(currency) = &self.currency {
            write!(f, "currency: {currency}. ")?;
        }
        if let Some(since) = &self.since {
            write!(f, "since {since}. ")?;
        }
        if let Some(until) = &self.until {
            write!(f, "until {until}. ")?;
        }
        if let Some(statuses) = &self.status {
            let statuses = statuses.iter().map(|s| s.to_string()).collect::<Vec<String>>().join(",");
            write!(f, "statuses: [{statuses}]. ")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModifyOrderRequest {
    pub new_customer_id: Option<String>,
    pub new_memo: Option<String>,
    pub new_total_price: Option<MicroTari>,
    pub new_currency: Option<String>,
    pub new_status: Option<OrderStatusType>,
}

impl ModifyOrderRequest {
    pub fn with_new_customer_id<S: Into<String>>(mut self, new_customer_id: S) -> Self {
        self.new_customer_id = Some(new_customer_id.into());
        self
    }

    pub fn with_new_memo<S: Into<String>>(mut self, new_memo: S) -> Self {
        self.new_memo = Some(new_memo.into());
        self
    }

    pub fn with_new_total_price(mut self, new_total_price: MicroTari) -> Self {
        self.new_total_price = Some(new_total_price);
        self
    }

    pub fn with_new_currency<S: Into<String>>(mut self, new_currency: S) -> Self {
        self.new_currency = Some(new_currency.into());
        self
    }

    pub fn with_new_status(mut self, new_status: OrderStatusType) -> Self {
        self.new_status = Some(new_status);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.new_customer_id.is_none() &&
            self.new_total_price.is_none() &&
            self.new_currency.is_none() &&
            self.new_status.is_none()
    }
}
