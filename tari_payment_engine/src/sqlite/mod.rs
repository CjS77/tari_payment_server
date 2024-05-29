//! SQLite database module for the Tari Payment Engine.
//!
//! As much of the logic needed for the database as possible is pushed into the SQL code for the database itself, in
//! the form of triggers. SQLite is not as powerful as Postgres in this regard, but the following actions are taken
//! care of by the database (and so is not present in the code). In case you're seeing unexpected behaviour, check
//! the database schema and triggers first.
//!
//! In particular, triggers are used to:
//! * Prevent DELETE queries on the `orders` table and `payments` tables. Orders and payments are never deleted, only
//!   cancelled.
//! * The `current_orders` and `total_orders` columns in the `user_accounts` table are maintained and updated
//!   automatically when orders are created or modified.
//! * Enforce that nonces are monotonically increasing every time that a user authenticates. Note that this check is
//!   partially replicated in the code, in case another backend implementation does _not_ do this. It's better to do
//!   this twice than accidentally never at all.
//!
//! ## Audit logs
//! The database maintains an audit log of all changes to the `orders` and `payments` tables. Any INSERT, or UPDATE
//! queries are logged to the `orders_log` or `payments_log` tables (DELETES are forbidden) through their
//! respective triggers.
//!
//! We also track any changes to the authorized wallet table in the `wallet_auth_log` table.
//!
//! General access audits are tracked in the application logs. See [`README.md`] for more information.
mod sqlite_impl;

pub mod db;
pub use sqlite_impl::SqliteDatabase;
