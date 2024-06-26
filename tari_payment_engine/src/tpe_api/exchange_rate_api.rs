//! The ExchangeRateApi trait defines the behaviour for managing exchange rates between Tari and other currencies,
//! for the cases when storefronts don't permit the use of custom currencies

use std::fmt::Debug;

use crate::{
    tpe_api::exchange_objects::ExchangeRate,
    traits::{ExchangeRateError, ExchangeRates},
};

pub struct ExchangeRateApi<B> {
    db: B,
}

impl<B> Debug for ExchangeRateApi<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExchangeRateApi")
    }
}

impl<B> ExchangeRateApi<B>
where B: ExchangeRates
{
    pub fn new(db: B) -> Self {
        Self { db }
    }

    pub async fn fetch_last_rate(&self, currency: &str) -> Result<ExchangeRate, ExchangeRateError> {
        self.db.fetch_last_rate(currency).await
    }

    pub async fn set_exchange_rate(&self, rate: &ExchangeRate) -> Result<(), ExchangeRateError> {
        self.db.set_exchange_rate(rate).await
    }
}
