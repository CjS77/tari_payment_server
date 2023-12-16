use crate::db::errors::DatabaseError;
use log::info;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use std::env;

const SQLITE_DB_URL: &str = "sqlite://data/tari_store.db";

pub fn db_url() -> String {
    let result = env::var("SPG_DATABASE_URL").unwrap_or_else(|_| {
        info!("SPG_DATABASE_URL is not set. Using the default.");
        SQLITE_DB_URL.to_string()
    });
    info!("Using database URL: {result}");
    result
}

pub async fn new_pool() -> Result<SqlitePool, DatabaseError> {
    let url = db_url();
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(url.as_str())
        .await?;
    Ok(pool)
}

pub mod orders;
pub mod transfers;
