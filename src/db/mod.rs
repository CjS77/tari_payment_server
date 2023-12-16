pub mod errors;
pub mod models;

mod common;

#[cfg(all(feature = "sqlite", feature = "postgres"))]
compile_error!("feature \"sqlite\" and feature \"postgres\" cannot be enabled at the same time");

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "postgres")]
mod postgres;

pub use common::{Database, InsertResult};
