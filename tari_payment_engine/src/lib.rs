//! Tari Payment Engine
//!
//! The Tari Payment Engine is a service that allows merchants to accept Tari as a payment method for goods and service.
//! This library contains the core logic for the payment engine. It is provider-agnostic.
//!
//! The library contains several submodules::
//! 1. The payment engine public API ([`mod@tpe_api`]). This provides the public-facing functionality of the payment
//!    engine. It is responsible for managing orders, authentication, payments and accounts. Specific backends (e.g.
//!    Postgres or SQLite) need to implement the traits in this module in order to act as a backend for the Tari Payment
//!    Server.
//! 2. Sqlite database implementation ([`mod@sqlite`]). You should never need to access the database directly.
//!    Instead, use the public API provided by the payment engine.
//! 3. The [`mod@db_types`] module defined the data types used in the database.
//! 4. The [`mod@events`] module defines the events that can be subscribed to. These events are emitted when certain actions
//! occur within the payment engine. For example, when a new order is created, an `OrderCreated` event is emitted.
//! A simple Pub-Sub mechanism is used so that you can easily hook into these events and perform custom actions.
//! 5. The [`mod@traits`] module the public contract specification that backends must implement in order to be used by the
//!   payment engine.

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
mod postgres;

pub mod db_types;
pub mod events;
pub mod helpers;
pub mod tpe_api;

pub mod traits;

#[cfg(any(feature = "test_utils", test))]
pub mod test_utils;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteDatabase;
pub use tpe_api::{
    accounts_api::AccountApi,
    auth_api::AuthApi,
    order_flow_api::OrderFlowApi,
    order_objects,
    wallet_api::WalletAuthApi,
};
