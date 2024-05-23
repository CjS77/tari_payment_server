//! SQLite database module for the Tari Payment Engine.

//!
mod sqlite_impl;

pub mod db;
pub use sqlite_impl::SqliteDatabase;
