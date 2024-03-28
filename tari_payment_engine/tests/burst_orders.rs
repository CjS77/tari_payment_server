use std::time::Duration;

use log::*;
use tari_payment_engine::{db_types::*, test_utils::prepare_env::prepare_test_env, OrderManagerApi, SqliteDatabase};
use tokio::runtime::Runtime;

const NUM_ORDERS: u64 = 20;
const RATE: u64 = 100; // orders per second

#[test]
fn burst_orders() {
    info!("🚀️ Starting order injection test");

    let sys = Runtime::new().unwrap();

    let delay = Duration::from_millis(1000 / RATE);

    sys.block_on(async move {
        let url = "sqlite://../data/test_burst_orders.db";
        prepare_test_env(url).await;
        let db = SqliteDatabase::new_with_url(url, 5).await.expect("Error creating database");
        let api = OrderManagerApi::new(db);

        let mut timer = tokio::time::interval(delay);
        info!("🚀️ Injecting {NUM_ORDERS} orders");
        for i in 0..NUM_ORDERS {
            timer.tick().await;
            let cid = ((i + 1) % 5).to_string();
            #[allow(clippy::cast_possible_wrap)]
            let price = MicroTari::from(1_000_000 * (i + 1) as i64);
            let order_id = OrderId::from(format!("oid-2024/{}-burstorder", i * 100));
            let new_order = NewOrder::new(order_id, cid, price);
            if let Err(e) = api.process_new_order(new_order).await {
                panic!("Error processing order {i}: {e}");
            }
        }
    });
    info!("🚀️ test complete");
}
