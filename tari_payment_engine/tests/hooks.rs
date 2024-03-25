use crate::support::prepare_env::{prepare_test_env, random_db_path};
use futures_util::FutureExt;
use log::*;
use sqlx::migrate::MigrateDatabase;
use sqlx::Sqlite;
use std::str::FromStr;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::db_types::{MicroTari, NewOrder, NewPayment, OrderId};
use tari_payment_engine::{OrderManagerApi, PaymentGatewayDatabase, SqliteDatabase};
use tokio::runtime::Runtime;

mod support;

async fn setup() -> OrderManagerApi<SqliteDatabase> {
    let url = random_db_path();
    prepare_test_env(&url).await;
    let db = SqliteDatabase::new_with_url(&url)
        .await
        .expect("Error creating database");
    OrderManagerApi::new(db)
}

async fn tear_down(mut api: OrderManagerApi<SqliteDatabase>) {
    if let Err(e) = api.db_mut().close().await {
        error!("🚀️ Failed to close database: {e}");
    }
    Sqlite::drop_database(api.db().url()).await.unwrap();
}

#[derive(Default, Clone)]
struct HookCalled {
    called: Arc<AtomicI32>,
}

impl HookCalled {
    pub fn called(&self) {
        let _ = self
            .called
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn count(&self) -> i32 {
        self.called.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[test]
fn on_order_created() {
    dotenvy::from_filename(".env.test").ok();
    let _ = env_logger::try_init();
    let rt = Runtime::new().unwrap();
    let event = HookCalled::default();
    let event_copy = event.clone();
    rt.block_on(async move {
        let mut api = setup().await;
        api.add_order_created_hook(Box::new(move |order| {
            info!("🪝️ {order:?}");
            event_copy.called();
            let fut = Box::pin(async {});
            fut.boxed_local()
        }));
        let id = OrderId::from_str("order1001").unwrap();
        let order = NewOrder::new(id, "alice".into(), MicroTari::from(1_000_000));
        let _ = api
            .process_new_order(order)
            .await
            .expect("Error processing order");
        let id = OrderId::from_str("order1002").unwrap();
        let order = NewOrder::new(id, "bob".into(), MicroTari::from(1_000_000));
        let _ = api
            .process_new_order(order)
            .await
            .expect("Error processing order");
        tear_down(api).await;
    });
    assert_eq!(event.count(), 2);
    info!("🪝️ test complete");
}

#[test]
fn on_payment_created() {
    dotenvy::from_filename(".env.test").ok();
    let _ = env_logger::try_init();
    let rt = Runtime::new().unwrap();
    let event = HookCalled::default();
    let event_copy = event.clone();
    rt.block_on(async move {
        let mut api = setup().await;
        api.add_payment_created_hook(Box::new(move |payment| {
            info!("🪝️ {payment:?}");
            event_copy.called();
            let fut = Box::pin(async {});
            fut.boxed_local()
        }));
        let sender = TariAddress::from_str(
            "6829578d62ddcba2191178287307a07dc8244af92b6bebc2b83ee41a40880e4897",
        )
        .unwrap();
        let amt = MicroTari::from(1_000_000);
        let payment = NewPayment::new(sender.clone(), amt, "tx1".into());
        let _ = api
            .process_new_payment(payment)
            .await
            .expect("Error processing payment");
        let payment = NewPayment::new(sender, amt, "tx2".into());
        let _ = api
            .process_new_payment(payment)
            .await
            .expect("Error processing payment");
        tear_down(api).await;
    });
    assert_eq!(event.count(), 2);
    info!("🪝️ test complete");
}
