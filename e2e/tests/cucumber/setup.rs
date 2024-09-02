use std::{collections::HashMap, net::IpAddr};

use chrono::{TimeZone, Utc};
use cucumber::{gherkin::Step, given, then};
use log::{debug, info, warn};
use tari_common_types::{tari_address::TariAddress, types::PrivateKey};
use tari_jwt::{
    jwt_compact::{AlgorithmExt, Claims, Header},
    tari_crypto::tari_utilities::hex::Hex,
    Ristretto256,
    Ristretto256SigningKey,
};
use tari_payment_engine::{
    db_types::{LoginToken, NewOrder, NewPayment, OrderId, Role},
    traits::{AuthManagement, NewWalletInfo, PaymentGatewayDatabase, WalletManagement},
};
use tpg_common::MicroTari;

use crate::cucumber::TPGWorld;

fn seed_orders() -> [NewOrder; 5] {
    [
        NewOrder {
            order_id: OrderId::new("1"),
            currency: "XTR".into(),
            customer_id: "alice".into(),
            memo: Some("Manually inserted by Keith".into()),
            address: "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt".parse().ok(),
            total_price: MicroTari::from_tari(100),
            original_price: None,
            created_at: Utc.with_ymd_and_hms(2024, 3, 10, 15, 0, 0).unwrap(),
        },
        NewOrder {
            order_id: OrderId::new("2"),
            currency: "XTR".into(),
            customer_id: "bob".into(),
            memo: Some("Manually inserted by Charlie".into()),
            address: "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp".parse().ok(),
            total_price: MicroTari::from_tari(200),
            original_price: None,
            created_at: Utc.with_ymd_and_hms(2024, 3, 10, 15, 30, 0).unwrap(),
        },
        NewOrder {
            order_id: OrderId::new("3"),
            currency: "XTR".into(),
            customer_id: "alice".into(),
            memo: Some("Manually inserted by Sam".into()),
            address: "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt".parse().ok(),
            total_price: MicroTari::from_tari(65),
            original_price: None,
            created_at: Utc.with_ymd_and_hms(2024, 3, 11, 16, 0, 0).unwrap(),
        },
        NewOrder {
            order_id: OrderId::new("4"),
            currency: "XTR".into(),
            customer_id: "bob".into(),
            memo: Some("Manually inserted by Ray".into()),
            address: "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp".parse().ok(),
            total_price: MicroTari::from_tari(350),
            original_price: None,
            created_at: Utc.with_ymd_and_hms(2024, 3, 11, 17, 0, 0).unwrap(),
        },
        NewOrder {
            order_id: OrderId::new("5"),
            currency: "XMR".into(),
            customer_id: "admin".into(),
            memo: Some("Manually inserted by Charlie".into()),
            address: "14sa5AzjqqrzfiyqGkajoNcFrqkCK7syB4rvNNL65f2PjLD".parse().ok(),
            total_price: MicroTari::from_tari(25),
            original_price: None,
            created_at: Utc.with_ymd_and_hms(2024, 3, 12, 18, 0, 0).unwrap(),
        },
    ]
}

fn seed_payments() -> [NewPayment; 5] {
    [
        NewPayment {
            sender: "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt".parse().unwrap(), // Alice
            amount: MicroTari::from_tari(15),
            txid: "alicepayment001".to_string(),
            memo: None,
            order_id: None,
        },
        NewPayment {
            sender: "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt".parse().unwrap(), // Alice
            amount: MicroTari::from_tari(100),
            txid: "alicepayment002".to_string(),
            memo: None,
            order_id: None,
        },
        NewPayment {
            sender: "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp".parse().unwrap(), // Bob
            amount: MicroTari::from_tari(50),
            txid: "bobpayment001".to_string(),
            memo: None,
            order_id: None,
        },
        NewPayment {
            sender: "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp".parse().unwrap(), // Bob
            amount: MicroTari::from_tari(500),
            txid: "bobpayment002".to_string(),
            memo: None,
            order_id: None,
        },
        NewPayment {
            sender: "142Eyn9FMCsBVRsFBc2zqfgBxPTTpX9dYjtrPABa9whREdia".parse().unwrap(), // Anon
            amount: MicroTari::from_tari(700),
            txid: "anonpayment001".to_string(),
            memo: None,
            order_id: None,
        },
    ]
}

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub username: String,
    pub secret: PrivateKey,
    pub address: TariAddress,
    pub roles: Vec<Role>,
}

fn super_admin() -> UserInfo {
    UserInfo {
        username: "Super".into(),
        secret: PrivateKey::from_hex("5b1488f90c3385b0a4d3ab9f6992f2592d35f77a57655160a9236ffadb78260c").unwrap(),
        address: TariAddress::from_base58("14t3efXHQphjE8GdVhSzxZH8VWeVphfqjiXsUvVEpTJJBA").unwrap(),
        roles: vec![Role::SuperAdmin],
    }
}

pub struct SuperAdmin(UserInfo);

impl SuperAdmin {
    pub fn new() -> Self {
        Self(super_admin())
    }

    pub fn token(&self, nonce: u64) -> String {
        let claims = LoginToken { address: self.0.address.clone(), nonce, desired_roles: vec![Role::SuperAdmin] };
        let claims = Claims::new(claims);
        let header = Header::empty().with_token_type("JWT");
        Ristretto256.token(&header, &claims, &Ristretto256SigningKey(self.0.secret.clone())).unwrap()
    }
}

pub struct SeedUsers {
    pub users: HashMap<&'static str, UserInfo>,
}

impl SeedUsers {
    pub fn new() -> Self {
        let mut users = HashMap::with_capacity(3);
        users.insert("Admin", UserInfo {
            username: "Admin".into(),
            secret: PrivateKey::from_hex("6fd9d9a5836eaf71c396c9af1915fe990c4c396b0c6f57c19db968af9d9ffd04").unwrap(),
            address: TariAddress::from_base58("14sa5AzjqqrzfiyqGkajoNcFrqkCK7syB4rvNNL65f2PjLD").unwrap(),
            roles: vec![Role::User, Role::Write, Role::ReadAll],
        });
        users.insert("Alice", UserInfo {
            username: "Alice".into(),
            secret: PrivateKey::from_hex("c63b5b436d007bec0566bff2f3512f3f962a6d43161fe48616e8dad58fd2b80d").unwrap(),
            address: TariAddress::from_base58("14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt").unwrap(),
            roles: vec![Role::User],
        });
        users.insert("Bob", UserInfo {
            username: "Bob".into(),
            secret: PrivateKey::from_hex("6ee8a6d1078755bfebd91751bd5d2fb76544ab49f23fe61d5c9a2857b7eea503").unwrap(),
            address: TariAddress::from_base58("14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp").unwrap(),
            roles: vec![Role::User],
        });
        Self { users }
    }

    pub fn user(&self, name: &str) -> &UserInfo {
        self.users.get(name).expect("User not found")
    }

    pub fn token_for(&self, name: &str, nonce: u64, roles: Vec<Role>) -> String {
        let user = self.user(name);
        let claims = LoginToken { address: user.address.clone(), nonce, desired_roles: roles };
        let claims = Claims::new(claims);
        let header = Header::empty().with_token_type("JWT");
        Ristretto256.token(&header, &claims, &Ristretto256SigningKey(user.secret.clone())).unwrap()
    }
}

#[given("a database with some accounts")]
async fn fresh_database(world: &mut TPGWorld) {
    world.start_database().await;
    let db = world.database();
    for order in seed_orders() {
        db.process_new_order_for_customer(order).await.unwrap();
    }
    world.start_server().await;
}

#[given("some payments are received")]
async fn payments_received(world: &mut TPGWorld) {
    let db = world.database();
    for payment in seed_payments() {
        db.process_new_payment_for_pubkey(payment).await.expect("Error persisting payment");
    }
}

#[given("the user is not logged in")]
async fn user_not_logged_in(_world: &mut TPGWorld) {
    // No-op
}

#[given("some role assignments")]
async fn roles_assignments(world: &mut TPGWorld) {
    setup_roles_assignments(world).await;
    info!("🌍️ Assigned initial role assignments");
}

#[then("everything is fine")]
async fn everything_is_fine(_world: &mut TPGWorld) {
    // No-op
}

#[then(expr = "pause for {int} ms")]
async fn pause_for_ms(_world: &mut TPGWorld, ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

#[given("a blank slate")]
async fn tabula_rasa(world: &mut TPGWorld) {
    world.start_database().await;
    world.start_server().await;
}

#[given("a super-admin user (Super)")]
async fn super_admin_user(world: &mut TPGWorld) {
    let admin = super_admin();
    let db = world.db.clone().unwrap();
    db.assign_roles(&admin.address, &admin.roles).await.unwrap();
    world.super_admin = Some(admin);
}

#[given(expr = "an authorized wallet with secret {word}")]
async fn authorize_wallet(world: &mut TPGWorld, step: &Step, secret: String) {
    let json = step.docstring().expect("JSON wallet specifier is missing");
    let info = serde_json::from_str::<NewWalletInfo>(&json).expect("Failed to parse wallet info");
    let secret = PrivateKey::from_hex(&secret).unwrap();
    world.wallets.insert(info.address.clone(), secret);
    let db = world.db.clone().unwrap();
    db.register_wallet(info).await.unwrap();
}

#[given("a server configuration")]
async fn server_configuration(world: &mut TPGWorld, step: &Step) {
    let Some(table) = step.table() else {
        warn!("Why specify a configuration step and then not supply a configuration?");
        return;
    };
    table.rows.iter().for_each(|row| {
        let key = row[0].as_str();
        let value = row[1].as_str();
        match key {
            "host" => world.config.host = value.into(),
            "port" => world.config.port = value.parse().expect("Invalid port number"),
            "shopify_api_key" => world.config.shopify_config.api_key = value.into(),
            "database_url" => world.config.database_url = value.into(),
            "shopify_whitelist" => {
                let ip = value.parse::<IpAddr>().expect("Invalid IP address");
                world.config.shopify_config.whitelist = Some(vec![ip])
            },
            "use_x_forwarded_for" => world.config.use_x_forwarded_for = value == "true",
            "use_forwarded" => world.config.use_forwarded = value == "true",
            _ => warn!("Unknown configuration key: {key}"),
        }
    });
    debug!("🌍️ Server configuration set. {:?}", world.config);
}

async fn setup_roles_assignments(world: &mut TPGWorld) {
    let users = SeedUsers::new();
    let db = world.db.clone().unwrap();
    for user in users.users.values() {
        if let Err(e) = db.assign_roles(&user.address, &user.roles).await {
            warn!("Error assigning roles to {}: {e}", user.username);
        }
    }
}
