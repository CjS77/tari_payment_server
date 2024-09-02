use std::{str::FromStr, time::Duration};

use log::*;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::NewPayment,
    events::EventProducers,
    test_utils::prepare_env::prepare_test_env,
    OrderFlowApi,
    SqliteDatabase,
};
use tokio::runtime::Runtime;
use tpg_common::MicroTari;

const NUM_TRANSFERS: u64 = 20;
const RATE: u64 = 100; // transfers per second

#[test]
fn burst_transfers() {
    info!("🚀️ Starting transfer burst test");

    let sys = Runtime::new().unwrap();

    let delay = Duration::from_millis(1000 / RATE);

    sys.block_on(async move {
        let url = "sqlite://../data/test_burst_transfers.db";
        prepare_test_env(url).await;
        let db = SqliteDatabase::new_with_url(url, 5).await.expect("Error creating database");
        let api = OrderFlowApi::new(db, EventProducers::default());
        let pk =
            TariAddress::from_str("14s9vDTwrweZvWEgQ9gNhXXPX68DPXSSAHNFWYEPi5JsBQY").expect("Not a valid Tari address");

        let mut timer = tokio::time::interval(delay);
        for i in 0..NUM_TRANSFERS {
            timer.tick().await;
            #[allow(clippy::cast_possible_wrap)]
            let amount = MicroTari::from((i + 1) as i64 * 1_000_000);

            let payment = NewPayment::new(pk.clone(), amount, format!("taritx-00-{i}"));
            let _res = api.process_new_payment(payment).await.expect("Error processing payment");
        }
    });
}
