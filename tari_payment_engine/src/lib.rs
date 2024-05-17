//! Tari Payment Engine
//!
//! The Tari Payment Engine is a service that allows merchants to accept Tari as a payment method for goods and service.
//! This library contains the core logic for the payment engine. It is provider-agnostic.
//!
//! The library is divided into two main sections:
//! 1. Database management and control ([`mod@db`]). Currently, Sqlite and Postgres are the two supported backends. You
//!    should never    need to access the database directly. Instead, use the public API provided by the payment engine.
//!    The exception is the data types used in the database. These are defined in the `db_types` module and are public.
//! 2. The payment engine public API ([`mod@tpe_api`]). This provides the public-facing functionality of the payment
//!    engine. It is responsible for managing orders, authentication, payments and accounts. Specific backends (e.g.
//!    Postgres or SQLite) need to implement the traits in this module in order to act as a backend for the Tari Payment
//!    Server.
//!
//! The engine also provides a set of events that can be subscribed to. These events are emitted when certain actions
//! occur within the payment engine. For example, when a new order is created, an `OrderCreated` event is emitted.
//! A simple Actor framework is used so that you can easily hook into these events and perform custom actions.
mod db;

pub mod db_types;
pub mod events;
pub mod helpers;
mod tpe_api;

#[cfg(any(feature = "test_utils", test))]
pub mod test_utils;

#[cfg(feature = "sqlite")]
pub use db::sqlite::SqliteDatabase;
pub use db::traits::{
    AccountManagement,
    AuthManagement,
    InsertOrderResult,
    InsertPaymentResult,
    OrderManagement,
    PaymentGatewayDatabase,
};
pub use tpe_api::{
    accounts_api::AccountApi,
    auth_api::AuthApi,
    errors::{AuthApiError, OrderManagerError},
    order_flow_api::OrderFlowApi,
    order_objects,
};
