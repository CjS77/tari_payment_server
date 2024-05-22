use chrono::{DateTime, Utc};
use log::trace;
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};
use tari_payment_engine::db_types::{MicroTari, NewOrder, OrderId};
use tpg_common::TARI_CURRENCY_CODE_LOWER;

use crate::errors::OrderConversionError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShopifyOrder {
    pub id: i64,
    pub token: String,
    pub cart_token: String,
    pub email: String,
    pub buyer_accepts_marketing: bool,
    pub created_at: String,
    pub updated_at: String,
    pub note: Option<String>,
    pub currency: String,
    pub completed_at: Option<String>,
    pub closed_at: Option<String>,
    pub user_id: Option<i64>,
    pub name: String,
    pub source_name: String,
    pub presentment_currency: String,
    pub total_discounts: String,
    pub total_line_items_price: String,
    pub total_price: String,
    pub total_tax: String,
    pub subtotal_price: String,
    pub customer: Customer,
}

impl TryFrom<ShopifyOrder> for NewOrder {
    type Error = OrderConversionError;

    fn try_from(value: ShopifyOrder) -> Result<Self, Self::Error> {
        trace!("Converting ShopifyOrder to NewOrder: {:?}", value);
        if value.currency.as_str().to_lowercase() != TARI_CURRENCY_CODE_LOWER {
            return Err(OrderConversionError(format!("Unsupported currency: {}", value.currency)));
        }
        let total_price = value
            .total_price
            .parse::<u64>()
            .map_err(|e| OrderConversionError(e.to_string()))
            .map(MicroTari::try_from)?
            .map_err(|e| OrderConversionError(e.to_string()))?;

        let timestamp = value.created_at.parse::<DateTime<Utc>>().map_err(|e| OrderConversionError(e.to_string()))?;
        let memo = value.note;
        let mut order = Self {
            order_id: OrderId(value.name),
            customer_id: value.email,
            currency: value.currency,
            memo,
            address: None,
            created_at: timestamp,
            total_price,
        };
        order.extract_address();
        Ok(order)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmailMarketingConsent {
    pub state: String,
    pub opt_in_level: Option<String>,
    pub consent_updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Customer {
    pub id: i64,
    pub email: String,
    pub accepts_marketing: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub orders_count: i64,
    pub state: String,
    pub total_spent: String,
    pub last_order_id: Option<String>,
    pub note: Option<String>,
    pub verified_email: bool,
    pub tax_exempt: bool,
    pub tags: String,
    pub last_order_name: Option<String>,
    pub currency: String,
    pub phone: Option<String>,
    pub email_marketing_consent: EmailMarketingConsent,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderBuilder {
    token: Option<String>,
    cart_token: Option<String>,
    email: Option<String>,
    buyer_accepts_marketing: Option<bool>,
    created_at: Option<String>,
    updated_at: Option<String>,
    note: Option<String>,
    currency: Option<String>,
    completed_at: Option<String>,
    closed_at: Option<String>,
    user_id: Option<i64>,
    name: Option<String>,
    source_name: Option<String>,
    presentment_currency: Option<String>,
    total_discounts: Option<String>,
    total_line_items_price: Option<String>,
    total_price: Option<String>,
    total_tax: Option<String>,
    subtotal_price: Option<String>,
    customer: Option<Customer>,
}

impl OrderBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn random_order() -> ShopifyOrder {
        OrderBuilder::new().build()
    }

    pub fn custom_order(note: String, price: &str) -> ShopifyOrder {
        let mut order = OrderBuilder::new();
        order.note(note).total_price(price.to_string());
        order.build()
    }

    pub fn email(&mut self, email: String) -> &mut Self {
        self.email = Some(email);
        self
    }

    pub fn created_at(&mut self, created_at: String) -> &mut Self {
        self.created_at = Some(created_at);
        self
    }

    pub fn updated_at(&mut self, updated_at: String) -> &mut Self {
        self.updated_at = Some(updated_at);
        self
    }

    pub fn note(&mut self, note: String) -> &mut Self {
        self.note = Some(note);
        self
    }

    pub fn currency(&mut self, currency: String) -> &mut Self {
        self.currency = Some(currency);
        self
    }

    pub fn closed_at(&mut self, closed_at: String) -> &mut Self {
        self.closed_at = Some(closed_at);
        self
    }

    pub fn user_id(&mut self, user_id: i64) -> &mut Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn name(&mut self, name: String) -> &mut Self {
        self.name = Some(name);
        self
    }

    pub fn presentment_currency(&mut self, presentment_currency: String) -> &mut Self {
        self.presentment_currency = Some(presentment_currency);
        self
    }

    pub fn total_discounts(&mut self, total_discounts: String) -> &mut Self {
        self.total_discounts = Some(total_discounts);
        self
    }

    pub fn total_line_items_price(&mut self, total_line_items_price: String) -> &mut Self {
        self.total_line_items_price = Some(total_line_items_price);
        self
    }

    pub fn total_price(&mut self, total_price: String) -> &mut Self {
        self.total_price = Some(total_price);
        self
    }

    pub fn total_tax(&mut self, total_tax: String) -> &mut Self {
        self.total_tax = Some(total_tax);
        self
    }

    pub fn subtotal_price(&mut self, subtotal_price: String) -> &mut Self {
        self.subtotal_price = Some(subtotal_price);
        self
    }

    pub fn customer(&mut self, customer: Customer) -> &mut Self {
        self.customer = Some(customer);
        self
    }

    pub fn build(self) -> ShopifyOrder {
        let mut rng = rand::thread_rng();
        #[allow(clippy::cast_possible_wrap)]
        let id = (rng.next_u64() >> 1) as i64;
        ShopifyOrder {
            id,
            token: self.token.unwrap_or_else(|| rng.next_u64().to_string()),
            cart_token: self.cart_token.unwrap_or_else(|| format!("{:x}", rng.next_u64())),
            email: self.email.unwrap_or_else(|| format!("{}@example.com", rng.gen_range(0..1000))),
            buyer_accepts_marketing: self.buyer_accepts_marketing.unwrap_or_default(),
            created_at: self.created_at.unwrap_or_else(|| Utc::now().to_rfc3339()),
            updated_at: self.updated_at.unwrap_or_else(|| Utc::now().to_rfc3339()),
            note: self.note,
            currency: self.currency.unwrap_or_else(|| "XTR".to_string()),
            completed_at: self.completed_at,
            closed_at: self.closed_at,
            user_id: self.user_id,
            name: self.name.unwrap_or_default(),
            source_name: self.source_name.unwrap_or_default(),
            presentment_currency: self.presentment_currency.unwrap_or_else(|| "XTR".to_string()),
            total_discounts: self.total_discounts.unwrap_or_default(),
            total_line_items_price: self.total_line_items_price.unwrap_or_default(),
            total_price: self.total_price.unwrap_or_else(|| format!("{}", rng.gen_range(1_000..250_000) * 1000)),
            total_tax: self.total_tax.unwrap_or_default(),
            subtotal_price: self.subtotal_price.unwrap_or_default(),
            customer: self.customer.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deserialize_new_order() {
        let order = include_str!("./test_assets/new_order.json");
        let order: ShopifyOrder = serde_json::from_str(order).unwrap();
        assert_eq!(order.id, 981820079255243500);
        assert_eq!(order.token, "123123123");
        assert_eq!(order.total_price, "398.00");
        assert_eq!(order.customer.id, 603851970716743400);
    }
}
