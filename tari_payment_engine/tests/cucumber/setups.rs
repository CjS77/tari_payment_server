use cucumber::given;

use crate::cucumber::{shopify_world::OrderManagementSystem, ShopifyWorld};

#[given("a fresh install")]
async fn fresh_database(world: &mut ShopifyWorld) {
    let system = OrderManagementSystem::new().await;
    world.system = Some(system);
}
