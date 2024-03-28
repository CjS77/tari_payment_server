mod cucumber;

use ::cucumber::{codegen::LocalBoxFuture, event::ScenarioFinished, gherkin, writer, World};
use futures_util::FutureExt;
use log::*;
use tokio::runtime::Runtime;

use crate::cucumber::TPGWorld;

fn main() {
    dotenvy::from_filename(".env.test").ok();
    env_logger::init();
    let sys = Runtime::new().unwrap();
    sys.block_on(
        TPGWorld::cucumber()
            .with_writer(writer::Libtest::or_basic())
            .after(|_f, _r, scenario, ev, w| post_test_hook(scenario, ev, w))
            .run("tests/features"),
    );
    info!("🚀️ Tests complete");
}

fn post_test_hook<'a>(
    scenario: &'a gherkin::Scenario,
    _ev: &'a ScenarioFinished,
    world: Option<&'a mut TPGWorld>,
) -> LocalBoxFuture<'a, ()> {
    let fut = async move {
        debug!("🚀️ After-scenario hook running for \"{}\"", scenario.name);
        if let Some(w) = world {
            if let Some(h) = w.server_handle.take() {
                info!("🚀️ Stopping server");
                h.stop(true).await;
                info!("🚀️ Server stopped");
            }
        }
    };
    fut.boxed_local()
}
