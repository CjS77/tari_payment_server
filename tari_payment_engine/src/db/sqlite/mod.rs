pub mod db;
mod errors;

pub mod auth;
pub mod orders;
pub mod transfers;
pub mod user_accounts;

use std::env;

pub use errors::SqliteDatabaseError;
use log::info;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

const SQLITE_DB_URL: &str = "sqlite://data/tari_store.db";

pub fn db_url() -> String {
    let result = env::var("SPG_DATABASE_URL").unwrap_or_else(|_| {
        info!("SPG_DATABASE_URL is not set. Using the default.");
        SQLITE_DB_URL.to_string()
    });
    info!("Using database URL: {result}");
    result
}

pub async fn new_pool(url: &str, max_connections: u32) -> Result<SqlitePool, SqliteDatabaseError> {
    let pool = SqlitePoolOptions::new().max_connections(max_connections).connect(url).await?;
    Ok(pool)
}
