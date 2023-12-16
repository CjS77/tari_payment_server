use crate::support::prepare_env::prepare_test_env;
use actix::dev::SendError;
use actix::Actor;
use actix::{System};
use log::*;
use shopify_payment_gateway::order_matcher::payment_watcher::PaymentWatcher;
use std::time::Duration;
use crate::support::transfers::random_transfer;

mod support;

const NUM_TRANSFERS: u64 = 10;
const RATE: u64 = 100; // transfers per second

#[test]
fn burst_transfers() {
    info!("🚀 Starting transfer burst test");

    let sys = System::new();

    let delay = Duration::from_millis(1000 / RATE);

    sys.block_on(async move {
        prepare_test_env().await;
        let payment_watcher = PaymentWatcher.start();

        let mut timer = tokio::time::interval(delay);
        for _ in 0..NUM_TRANSFERS {
            timer.tick().await;
            let new_transfer = random_transfer();
            match payment_watcher.try_send(new_transfer) {
                Err(SendError::Full(_)) => {
                    panic!("🚀 Payment watcher message queue is full");
                }
                Err(SendError::Closed(_)) => {
                    panic!("🚀 Payment watcher message queue is closed");
                }
                Ok(()) => {
                    debug!("🚀 Payment watcher message was sent ok.");
                }
            }
        }
    });
    info!("🚀 Payment watcher test complete");
}
