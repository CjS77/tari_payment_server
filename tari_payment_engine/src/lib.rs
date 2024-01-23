//! Tari Payment Engine
//!
//! The Tari Payment Engine is a service that allows merchants to accept Tari as a payment method for goods and service.
//! This library contains the core logic for the payment engine. It is provider-agnostic.
//!
//! The library is divided into two main sections:
//! 1. Database management and control. Currently, Sqlite and Postgres are the two supported backends. You should never
//!    need to access the database directly. Instead, use the public API provided by the payment engine.
//!    The exception is the data types used in the database. These are defined in the `db_types` module and are public.
//! 2. Order management and control. This is the core logic of the payment engine. It is responsible for managing orders,
//!    and exposes the public API for the payment engine.
//!
//! The engine also provides a set of events that can be subscribed to. These events are emitted when certain actions
//! occur within the payment engine. For example, when a new order is created, an `OrderCreated` event is emitted.
//! A simple Actor framework is used so that you can easily hook into these events and perform custom actions.
mod db;

mod address_extractor;
pub mod db_types;
pub mod events;
mod order_manager;

pub use db::common::{
    AccountManagement, InsertOrderResult, InsertPaymentResult, PaymentGatewayDatabase,
};
pub use order_manager::{api::OrderManagerApi, errors::OrderManagerError};

#[cfg(feature = "sqlite")]
pub use db::sqlite::db::SqliteDatabase;
