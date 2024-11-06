//! # SQLite Database methods
//!
//! This module contains "low-level" SQLite database interactions.
//!
//! All these interaction are maintained by simple functions (rather than stateful structs) that accept a
//! `&mut SqliteConnection` argument. Callers can obtain a connection from a pool,
//! or create an atomic transaction as the need arises and call through to the functions without any other changes.
use std::env;

use log::info;
use sqlx::{sqlite::SqlitePoolOptions, Error as SqlxError, SqlitePool};

pub mod accounts;
pub mod auth;
pub mod exchange_rates;
pub mod orders;
pub mod shopify;
pub mod transfers;
pub mod wallet_auth;

const SQLITE_DB_URL: &str = "sqlite://data/tari_store.db";

pub fn db_url() -> String {
    let result = env::var("TPG_DATABASE_URL").unwrap_or_else(|_| {
        info!("TPG_DATABASE_URL is not set. Using the default.");
        SQLITE_DB_URL.to_string()
    });
    info!("Using database URL: {result}");
    result
}

pub async fn new_pool(url: &str, max_connections: u32) -> Result<SqlitePool, SqlxError> {
    let pool = SqlitePoolOptions::new().max_connections(max_connections).connect(url).await?;
    Ok(pool)
}
