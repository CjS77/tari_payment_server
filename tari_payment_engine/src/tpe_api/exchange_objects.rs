use std::fmt::Display;

use chrono::{DateTime, Utc};
use sqlx::FromRow;
use tpg_common::MicroTari;

#[derive(Debug, Clone, FromRow)]
pub struct ExchangeRate {
    pub base_currency: String,
    /// the exchange rate, in hundredths of the base unit (i.e. rate = 100 is a 1:1 exchange rate)
    pub rate: i64,
    pub updated_at: DateTime<Utc>,
}

impl ExchangeRate {
    /// Create a new ExchangeRate object
    ///
    /// *NB* The rate is in hundreds of the base unit (i.e. rate = 100 is a 1:1 exchange rate)
    pub fn new(currency: String, rate: i64, updated_at: Option<DateTime<Utc>>) -> Self {
        let updated_at = updated_at.unwrap_or_else(Utc::now);
        Self { base_currency: currency, rate, updated_at }
    }

    /// Convert an amount in the base currency to Tari
    pub fn convert_to_tari(&self, amount: i64) -> MicroTari {
        MicroTari::from(amount * self.rate * 10_000)
    }
}

impl Display for ExchangeRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "1 {} => {} XTR", self.base_currency, self.rate / 100)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_exchange_rate() {
        // 1:1 exchange rate
        let rate = ExchangeRate::new("USD".to_string(), 100, None);
        assert_eq!(rate.convert_to_tari(100), MicroTari::from_tari(100));
        assert_eq!(format!("{rate}"), "1 USD => 1 XTR");

        // 1 XTR : 2c
        let rate = ExchangeRate::new("USD".to_string(), 5000, None);
        assert_eq!(rate.convert_to_tari(1), MicroTari::from_tari(50));
        assert_eq!(format!("{rate}"), "1 USD => 50 XTR");
    }
}
