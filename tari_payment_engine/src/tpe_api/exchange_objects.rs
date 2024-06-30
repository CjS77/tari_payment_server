use std::fmt::Display;

use chrono::{DateTime, Utc};
use sqlx::FromRow;
use tpg_common::MicroTari;

#[derive(Debug, Clone, FromRow)]
pub struct ExchangeRate {
    pub base_currency: String,
    /// the exchange rate, in hundredths of the base unit (i.e. how many Tari in one cent of the base currency)
    pub rate: MicroTari,
    pub updated_at: DateTime<Utc>,
}

impl ExchangeRate {
    /// Create a new ExchangeRate object
    ///
    /// *NB* The rate is in hundreds of the base unit (i.e. how many microTari in one cent of the base currency)
    pub fn new(currency: String, rate_per_cent: MicroTari, updated_at: Option<DateTime<Utc>>) -> Self {
        let updated_at = updated_at.unwrap_or_else(Utc::now);
        Self { base_currency: currency, rate: rate_per_cent, updated_at }
    }

    /// Create a new ExchangeRate object with a rate of 1 base unit per Tari
    pub fn parity(currency: String) -> Self {
        Self::new(currency, MicroTari::from(10_000), None)
    }

    /// Convert an amount in the base currency to Tari
    pub fn convert_to_tari(&self, amount: i64) -> MicroTari {
        self.rate * amount * 100
    }

    /// Convert an amount in base currency cents to Tari
    pub fn convert_to_tari_from_cents(&self, cents: i64) -> MicroTari {
        self.rate * cents
    }
}

impl Display for ExchangeRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "1 {} => {}", self.base_currency, self.rate * 100)
    }
}

impl Default for ExchangeRate {
    fn default() -> Self {
        Self { base_currency: "XTR".to_string(), rate: MicroTari::from(10_000), updated_at: Utc::now() }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_exchange_rate() {
        // 1:1 exchange rate
        let rate = ExchangeRate::default();
        assert_eq!(rate.convert_to_tari(5), MicroTari::from_tari(5));
        assert_eq!(rate.convert_to_tari_from_cents(50), MicroTari::from(500_000));
        assert_eq!(format!("{rate}"), "1 XTR => 1.000τ");

        // 5000 XTR/$
        let rate = ExchangeRate::new("USD".to_string(), MicroTari::from_tari(50), None);
        assert_eq!(rate.convert_to_tari(5), MicroTari::from_tari(25_000));
        assert_eq!(rate.convert_to_tari_from_cents(2), MicroTari::from_tari(100));
        assert_eq!(format!("{rate}"), "1 USD => 5000.000τ");

        // 1 XTR : 2c (1c => 500,000 microTari)
        let rate = ExchangeRate::new("USD".to_string(), MicroTari::from(500_000), None);
        assert_eq!(rate.convert_to_tari(1), MicroTari::from_tari(50));
        assert_eq!(format!("{rate}"), "1 USD => 50.000τ");

        // 1 XTR = $1
        let rate = ExchangeRate::parity("USD".to_string());
        assert_eq!(rate.convert_to_tari(1), MicroTari::from_tari(1));
        assert_eq!(format!("{rate}"), "1 USD => 1.000τ");
    }
}
