use sqlx::SqliteConnection;

use crate::{tpe_api::exchange_objects::ExchangeRate, traits::ExchangeRateError};

pub async fn fetch_last_rate(currency: &str, conn: &mut SqliteConnection) -> Result<ExchangeRate, ExchangeRateError> {
    let result = sqlx::query_as!(
        ExchangeRate,
        r#"SELECT
      base_currency,
      rate,
      updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
      FROM exchange_rates WHERE base_currency = $1 ORDER BY updated_at DESC LIMIT 1"#,
        currency
    )
    .fetch_optional(conn)
    .await
    .map_err(|e| ExchangeRateError::DatabaseError(e.to_string()))?
    .ok_or_else(|| ExchangeRateError::RateDoesNotExist(currency.to_string()))?;
    Ok(result)
}

pub async fn set_exchange_rate(rate: &ExchangeRate, conn: &mut SqliteConnection) -> Result<(), ExchangeRateError> {
    sqlx::query!(r#"INSERT INTO exchange_rates (base_currency, rate) VALUES ($1, $2)"#, rate.base_currency, rate.rate)
        .execute(conn)
        .await
        .map_err(|e| ExchangeRateError::DatabaseError(e.to_string()))?;
    Ok(())
}
