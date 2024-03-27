use cucumber::given;
use crate::cucumber::shopify_world::OrderManagementSystem;
use crate::cucumber::ShopifyWorld;

#[given("a fresh install")]
async fn fresh_database(world: &mut ShopifyWorld) {
    let system = OrderManagementSystem::new().await;
    world.system = Some(system);
}

#[given("a database with some accounts")]
async fn database_with_accounts(world: &mut ShopifyWorld) {
    let system = OrderManagementSystem::new().await;
    world.system = Some(system);
    let db = world.api().db();
    let _ = db.create_user_account("Alice", "pkAlice").await.expect("Error creating account");
    let _ = db.create_user_account("Bob", "pkBob").await.expect("Error creating account");
}

#[given("a signer with secret key 'xxxxx'")]

