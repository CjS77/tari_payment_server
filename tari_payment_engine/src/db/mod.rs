//! The `db` module defines the interface contracts for backend wanting to support the payment engine,
//! as well as specific backend implementations.
//!
//! * the [`traits`] module defines the interface contracts for any backend wanting to support the payment engine.
//! * the [`postgres`] module provides Postgres support for the Tari payment engine (TODO)
//! * the [`sqlite`] module provides SQLite support for the Tari payment engine.
pub mod traits;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "sqlite")]
pub mod sqlite;
