use crate::db_types::{Order, OrderId};

/// The `OrderManagement` trait defines the behaviour for querying information about orders in the database backend.
#[allow(async_fn_in_trait)]
pub trait OrderManagement {
    type Error: std::error::Error;

    async fn order_by_id(&self, order_id: &OrderId) -> Result<Option<Order>, Self::Error>;
}
