//! SQLite database module for the Tari Payment Engine.
//!
//! To make things easier to test and maintain, the implementation is split into 2 parts. A set of modules that define
//! methods that interact directly with the database, and a set of modules that implement the payment engine traits so
//! that SQLite can be used as a backend for the payment engine.
//!
//! For the most part, the trait implementations are simple wrappers around the database methods.
//! In some instances, Transaction control is implemented in the trait implementations, so that multiple calls to the
//! database can be rolled back if something fails in the call chain.
//!
//! To make this workable, all "lower level" database interaction are maintained by simple functions (rather than
//! stateful structs) that accept a `&mut SqliteConnection` argument. Callers can obtain a connection from a pool,
//! or create an atomic transaction as the need arises and call through to the functions without any other changes.
mod errors;

pub mod auth;
pub mod orders;
mod sqlite_database;
pub mod transfers;
pub mod user_accounts;

use std::env;

pub use errors::SqliteDatabaseError;
use log::info;
pub use sqlite_database::SqliteDatabase;
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
