use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use futures_util::future::join_all;
use log::*;
use tari_payment_engine::{
    db_types::*,
    events::EventProducers,
    test_utils::prepare_env::prepare_test_env,
    OrderFlowApi,
    SqliteDatabase,
};
use tokio::runtime::{Builder, Runtime};
use tpg_common::MicroTari;

const NUM_BATCHES: usize = 20;
const NUM_THREADS: usize = 5;
const RATE: u64 = 100; // orders per second

#[test]
fn burst_orders() {
    info!("🚀️ Starting order injection test");

    let sys = Builder::new_multi_thread().worker_threads(NUM_THREADS).enable_time().build().unwrap();

    let delay = Duration::from_millis(1000 / RATE * 5);

    let successes = Arc::new(AtomicU64::new(0));
    let s2 = successes.clone();
    let num_orders = NUM_BATCHES * NUM_THREADS;
    sys.block_on(async move {
        let url = "sqlite://../data/test_burst_orders.db";
        prepare_test_env(url).await;
        let db = SqliteDatabase::new_with_url(url, NUM_THREADS as u32).await.expect("Error creating database");

        info!("🚀️ Injecting {num_orders} orders");
        let mut tasks = Vec::with_capacity(NUM_BATCHES);
        for t in 0..NUM_THREADS {
            let db2 = db.clone();
            let s3 = s2.clone();
            let task = tokio::spawn(async move {
                let api = OrderFlowApi::new(db2, EventProducers::default());

                for i in 0..NUM_BATCHES {
                    let mut timer = tokio::time::interval(delay);
                    timer.tick().await;
                    let cid = ((i + 1) % 5).to_string();
                    #[allow(clippy::cast_possible_wrap)]
                    let price = MicroTari::from(1_000_000 * (i + 1) as i64);
                    let order_id = OrderId::from(format!("oid-2024/{}.{}-burstorder", t, i * 100));
                    let new_order = NewOrder::new(order_id, cid, price);
                    if let Err(e) = api.process_new_order(new_order, true, true).await {
                        panic!("Error processing order {i}: {e}");
                    } else {
                        s3.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
            tasks.push(task);
        }
        let results = join_all(tasks).await;
        assert!(results.iter().all(|r| r.is_ok()), "Not all threads completed happily");
    });
    let successes = successes.as_ref().load(Ordering::SeqCst);
    assert_eq!(successes, num_orders as u64);
    info!("🚀️ test complete");
}
