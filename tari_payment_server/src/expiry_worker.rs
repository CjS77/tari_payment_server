use chrono::Duration;
use log::*;
use tari_payment_engine::{db_types::Order, events::EventProducers, OrderFlowApi, SqliteDatabase};
use tokio::task::JoinHandle;

/// Starts the expiry worker. Do not await the returned JoinHandle, as it will run indefinitely.
pub fn start_expiry_worker(
    db: SqliteDatabase,
    producers: EventProducers,
    unclaimed_expiry: Duration,
    unpaid_expiry: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(60));
        let api = OrderFlowApi::new(db, producers);
        info!("🕰️ Unclaimed order expiry worker started");
        loop {
            timer.tick().await;
            info!("🕰️ Running unclaimed order expiry job");
            match api.expire_old_orders(unclaimed_expiry, unpaid_expiry).await {
                Ok(result) => {
                    info!("🕰️ {} orders expired", result.total_count());
                    debug!(
                        "🕰️ {} Expired unclaimed orders: {}",
                        result.unclaimed_count(),
                        order_list(&result.unclaimed)
                    );
                    debug!("🕰️ {} Expired unpaid orders: {}", result.unpaid_count(), order_list(&result.unpaid));
                },
                Err(e) => {
                    error!("🕰️ Error running unclaimed order expiry job: {e}");
                },
            }
        }
    })
}

fn order_list(orders: &[Order]) -> String {
    orders
        .iter()
        .map(|o| format!("[{}] order_id: {} cust_id: {}", o.id, o.order_id, o.customer_id))
        .collect::<Vec<String>>()
        .join(", ")
}
