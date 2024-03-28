mod cucumber;

use ::cucumber::{codegen::LocalBoxFuture, event::ScenarioFinished, gherkin, writer, World};
use futures_util::FutureExt;
use log::*;
use sqlx::{migrate::MigrateDatabase, Sqlite};
use tari_payment_engine::PaymentGatewayDatabase;
use tokio::runtime::Runtime;

use crate::cucumber::ShopifyWorld;

fn main() {
    dotenvy::from_filename(".env.test").ok();
    env_logger::init();
    let sys = Runtime::new().unwrap();
    sys.block_on(
        ShopifyWorld::cucumber()
            .with_writer(writer::Libtest::or_basic())
            .after(|_f, _r, scenario, ev, w| post_test_hook(scenario, ev, w))
            .run("tests/features"),
    );
    info!("🚀️ Tests complete");
}

fn post_test_hook<'a>(
    scenario: &'a gherkin::Scenario,
    ev: &'a ScenarioFinished,
    world: Option<&'a mut ShopifyWorld>,
) -> LocalBoxFuture<'a, ()> {
    let fut = async move {
        trace!("🚀️ After-scenario hook running for \"{}\"", scenario.name);
        if let Some(ShopifyWorld { system: Some(sys) }) = world {
            let db_path = sys.db_path.clone();
            match ev {
                ScenarioFinished::StepFailed(_, _, _) | ScenarioFinished::StepSkipped => {
                    error!("🚀️ Error in scenario, database retained: {db_path}");
                },
                ScenarioFinished::StepPassed => {
                    debug!("🚀️ Scenario complete, removing database: {db_path}");
                    if let Err(e) = sys.api.db_mut().close().await {
                        error!("🚀️ Failed to close database: {e}");
                    }
                    Sqlite::drop_database(&db_path).await.unwrap();
                },
                _ => trace!("🚀️ Unhandled event: {ev:?}"),
            }
        } else {
            warn!("🚀️ World was not specified. Cannot cleanup database.");
        }
        trace!("🚀️ After-scenario hook complete");
    };
    fut.boxed_local()
}
