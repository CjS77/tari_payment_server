//! # Tari payment engine public API
//!
//! The `tpe_api` module exposes the programmatic API for the Tari Payment engine.
//! The API is modular, so that clients of the API can pick and choose the functionality they want.
//! Or different parts (e.g. auth and orders) could be configured on different machines, or even use Sqlite for one and
//! Postgres for the other.
//!
//! * [`accounts_api`] provides methods for interacting with user accounts, including fetching order and payment
//!   histories, status, and metadata.
//! * [`auth_api`] manages nonce state for authentication tokens, and managing user [`Role`]s
//! * [`order_flow_api`] is the primary API for handling order and payment flows in response to merchant order events
//!   and wallet payment events.
//! * [`wallet_api`] provides methods for interacting with the hot wallet authorization and authentication.
//!
//! The other submodules in this module are support and utility functions and types.
//!
//! # API usage
//!
//! The pattern for using all the APIs is the same. An API instance is created by supplying a database backend that
//! implements the specific backend traits required by the API.
//!
//! For example, to create an API instance to query the accounts on the database:
//!
//! ```rust,ignore
//! use tari_common_types::tari_address::TariAddress;
//! use tari_payment_engine::{AccountApi, SqliteDatabase};
//! let db = SqliteDatabase::new_with_url(...).await?;
//! // SqliteDatabase implements AccountManagement
//! let api = AccountApi::new(db);
//! // use the api to access information
//! let account = api.account_by_address(&a_tari_address).await?;
//! ```

pub mod accounts_api;
pub mod auth_api;
pub mod order_flow_api;
pub mod order_objects;
pub mod payment_objects;

pub mod wallet_api;
