use thiserror::Error;

use crate::tpe_api::exchange_objects::ExchangeRate;

#[derive(Debug, Clone, Error)]
pub enum ExchangeRateError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("The requested exchange rate does not exist: {0}")]
    RateDoesNotExist(String),
}

#[allow(async_fn_in_trait)]
pub trait ExchangeRates {
    /// Fetch the last exchange rate for the given currency. If the rate does not exist, the error
    /// [`ExchangeRateError::RateDoesNotExist`] is returned.
    async fn fetch_last_rate(&self, currency: &str) -> Result<ExchangeRate, ExchangeRateError>;
    /// Save the exchange rate for the given currency to the backend storage
    async fn set_exchange_rate(&self, rate: &ExchangeRate) -> Result<(), ExchangeRateError>;
}
