use std::{collections::HashMap, convert::Into};

use chrono::{TimeZone, Utc};
use cucumber::{given, then};
use log::info;
use tari_common_types::{tari_address::TariAddress, types::PrivateKey};
use tari_jwt::{
    jwt_compact::{AlgorithmExt, Claims, Header},
    tari_crypto::tari_utilities::hex::Hex,
    Ristretto256,
    Ristretto256SigningKey,
};
use tari_payment_engine::{
    db_types::{LoginToken, MicroTari, NewOrder, OrderId, Role},
    AuthManagement,
    PaymentGatewayDatabase,
};

use crate::cucumber::TPGWorld;

fn seed_orders() -> [NewOrder; 5] {
    [
        NewOrder {
            order_id: OrderId::new("1"),
            currency: "XTR".into(),
            customer_id: "alice".into(),
            memo: Some("address: [b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d]".into()),
            address: None,
            total_price: MicroTari::from_tari(100),
            created_at: Utc.with_ymd_and_hms(2024, 3, 10, 15, 0, 0).unwrap(),
        },
        NewOrder {
            order_id: OrderId::new("2"),
            currency: "XTR".into(),
            customer_id: "bob".into(),
            memo: Some("address: [680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b]".into()),
            address: None,
            total_price: MicroTari::from_tari(200),
            created_at: Utc.with_ymd_and_hms(2024, 3, 10, 15, 30, 0).unwrap(),
        },
        NewOrder {
            order_id: OrderId::new("3"),
            currency: "XTR".into(),
            customer_id: "alice".into(),
            memo: Some("address: [b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d]".into()),
            address: None,
            total_price: MicroTari::from_tari(65),
            created_at: Utc.with_ymd_and_hms(2024, 3, 11, 16, 0, 0).unwrap(),
        },
        NewOrder {
            order_id: OrderId::new("4"),
            currency: "XTR".into(),
            customer_id: "bob".into(),
            memo: Some("address: [680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b]".into()),
            address: None,
            total_price: MicroTari::from_tari(350),
            created_at: Utc.with_ymd_and_hms(2024, 3, 11, 17, 0, 0).unwrap(),
        },
        NewOrder {
            order_id: OrderId::new("5"),
            currency: "XTR".into(),
            customer_id: "admin".into(),
            address: None,
            memo: Some("address: [aa3c076152c1ae44ae86585eeba1d348badb845d1cab5ef12db98fafb4fea55d6c]".into()),
            total_price: MicroTari::from_tari(25),
            created_at: Utc.with_ymd_and_hms(2024, 3, 12, 18, 0, 0).unwrap(),
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
        address: TariAddress::from_hex("02f671c8294931a6395b51a1f32921f429d22c1e34def8f9f81892034fe2963cf7").unwrap(),
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
            address: TariAddress::from_hex("aa3c076152c1ae44ae86585eeba1d348badb845d1cab5ef12db98fafb4fea55d6c")
                .unwrap(),
            roles: vec![Role::User, Role::Write, Role::ReadAll],
        });
        users.insert("Alice", UserInfo {
            username: "Alice".into(),
            secret: PrivateKey::from_hex("c63b5b436d007bec0566bff2f3512f3f962a6d43161fe48616e8dad58fd2b80d").unwrap(),
            address: TariAddress::from_hex("b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d")
                .unwrap(),
            roles: vec![Role::User],
        });
        users.insert("Bob", UserInfo {
            username: "Bob".into(),
            secret: PrivateKey::from_hex("6ee8a6d1078755bfebd91751bd5d2fb76544ab49f23fe61d5c9a2857b7eea503").unwrap(),
            address: TariAddress::from_hex("680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b")
                .unwrap(),
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
    for mut order in seed_orders() {
        order.extract_address();
        db.process_new_order_for_customer(order).await.unwrap();
    }
    world.start_server().await;
}

#[given("some role assignments")]
async fn roles_assignments(world: &mut TPGWorld) {
    setup_roles_assignments(world).await;
    info!("🌍️ Assigned initial role assignments");
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

async fn setup_roles_assignments(world: &mut TPGWorld) {
    let users = SeedUsers::new();
    let db = world.db.clone().unwrap();
    for user in users.users.values() {
        db.assign_roles(&user.address, &user.roles).await.unwrap();
    }
}
