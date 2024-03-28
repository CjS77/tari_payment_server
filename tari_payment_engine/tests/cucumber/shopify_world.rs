use cucumber::World;
use log::*;
use tari_payment_engine::{
    test_utils::prepare_env::{create_database, random_db_path, run_migrations},
    OrderManagerApi,
    SqliteDatabase,
};
use tokio::time::sleep;

#[derive(Default, Debug, World)]
pub struct ShopifyWorld {
    pub system: Option<OrderManagementSystem>,
}

#[derive(Debug)]
pub struct OrderManagementSystem {
    pub db_path: String,
    pub api: OrderManagerApi<SqliteDatabase>,
}

impl ShopifyWorld {
    pub fn api(&self) -> &OrderManagerApi<SqliteDatabase> {
        &self.system.as_ref().expect("OrderManagerApi not initialised").api
    }
}

impl OrderManagementSystem {
    pub async fn new() -> Self {
        let url = prepare_test_env().await;
        let db = SqliteDatabase::new_with_url(&url, 1).await.expect("Error creating connection to database");
        debug!("Created database: {url}");
        sleep(std::time::Duration::from_millis(50)).await;
        let api = OrderManagerApi::new(db);
        Self { db_path: url, api }
    }
}

pub async fn prepare_test_env() -> String {
    let path = random_db_path();
    create_database(&path).await;
    run_migrations(&path).await;
    path
}
