use crate::support::prepare_env::prepare_test_env;
use actix::Actor;
use actix::{System, SystemService};
use log::*;
use shopify_payment_gateway::order_matcher::order_watcher::OrderWatcher;
use shopify_payment_gateway::spg_server::new_order::OrderBuilder;
use shopify_payment_gateway::spg_server::new_order_service::NewOrderService;
use shopify_payment_gateway::spg_server::routes::dispatch_event_to_subscribers;
use std::time::Duration;

mod support;

const NUM_ORDERS: u64 = 10;
const RATE: u64 = 100; // orders per second

#[test]
fn burst_orders() {
    info!("🚀 Starting order injection test");

    let sys = System::new();

    let delay = Duration::from_millis(1000 / RATE);

    sys.block_on(async move {
        prepare_test_env().await;
        let addr = NewOrderService::from_registry();
        let _order_watcher = OrderWatcher::default().start();

        let mut timer = tokio::time::interval(delay);
        for _ in 0..NUM_ORDERS {
            timer.tick().await;
            let new_order = OrderBuilder::random_order();
            dispatch_event_to_subscribers(addr.clone(), new_order.clone()).unwrap();
        }
    });
    info!("🚀 test complete");
}
