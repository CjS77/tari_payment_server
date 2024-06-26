use std::str::FromStr;

use chrono::Duration;
use cucumber::{gherkin::Step, given, then, when};
use e2e::helpers::json_is_subset_of;
use log::*;
use reqwest::Method;
use shopify_tools::ShopifyOrder;
use tari_common_types::tari_address::TariAddress;
use tari_jwt::{
    jwt_compact::{AlgorithmExt, Claims, Header, UntrustedToken},
    Ristretto256,
    Ristretto256SigningKey,
};
use tari_payment_engine::{
    db_types::{NewPayment, OrderId, Role},
    events::{EventProducers, EventType},
    traits::{AccountManagement, AuthManagement, PaymentGatewayDatabase},
    OrderFlowApi,
};
use tari_payment_server::{
    auth::{build_jwt_signer, JwtClaims},
    data_objects::{PaymentNotification, TransactionConfirmationNotification},
};
use tokio::time::sleep;
use tpg_common::MicroTari;

use crate::cucumber::{
    setup::{SeedUsers, SuperAdmin},
    TPGWorld,
};

#[then("the server is running")]
async fn server_is_running(world: &mut TPGWorld) {
    let (code, body) = world.get("health").await;
    assert_eq!(code.as_u16(), 200);
    assert_eq!(body, "👍️\n");
}

#[when("I authenticate with the auth header")]
async fn authenticate_with_auth_header(world: &mut TPGWorld, step: &Step) {
    let (header, token) = extract_token(step.docstring());
    world.response = None;
    let req = world.request(Method::POST, "/auth", |req| req.header(header, token));
    let res = req.await;
    debug!("Got Response: {} {}", res.0, res.1);
    world.response = Some(res);
}

//             I receive a {int} {word} response with the message "Missing login token"
#[then(expr = "I receive a {int} {word} response with the message {string}")]
async fn receive_response(world: &mut TPGWorld, status: u16, text: String, message: String) {
    let (res_status, res_msg) = world.response.take().expect("No response received");
    assert_eq!(res_status, status, "Expected {status} {text} response, got {res_status}");
    assert!(res_msg.contains(&message), "Expected response to contain '{message}', got '{res_msg}'");
}

#[then(expr = "I receive a {int} {word} response")]
async fn receive_response_code(world: &mut TPGWorld, status: u16, text: String) {
    let (res_status, _res_msg) = world.response.clone().expect("No response received");
    assert_eq!(res_status, status, "Expected {status} {text} response, got {res_status}");
}

#[then(expr = "I receive a partial JSON response:")]
async fn receive_json_response(world: &mut TPGWorld, step: &Step) {
    let (_res_status, res_msg) = world.response.take().expect("No response received");
    let partial_match = step.docstring().expect("No expected response");
    assert!(
        json_is_subset_of(partial_match, res_msg.as_str()),
        "Expected response to be '{partial_match}', got '{res_msg}'"
    );
}

#[when(expr = "Super authenticates with nonce = {int}")]
async fn super_admin_auth(world: &mut TPGWorld, nonce: u64) {
    let admin = SuperAdmin::new();
    let token = admin.token(nonce);
    debug!("Token for Super-Admin: {token}");
    let (code, token) = world.request(Method::POST, "/auth", |req| req.header("tpg_auth_token", token)).await;
    assert_eq!(code.as_u16(), 200);
    world.access_token = Some(token);
}

// Alice authenticates with nonce = 1, and roles = [user, read_all, write]
#[when(expr = "{word} authenticates with nonce = {int} and roles = {string}")]
async fn user_auth(world: &mut TPGWorld, user: String, nonce: u64, roles: String) {
    let users = SeedUsers::new();
    let roles = extract_roles(roles.as_str());
    let token = users.token_for(user.as_str(), nonce, roles);
    debug!("Token for {user}: {token}");
    let (code, token) = world.request(Method::POST, "/auth", |req| req.header("tpg_auth_token", token)).await;
    world.response = Some((code, token.clone()));
    world.logged_in = code == 200;
    if world.logged_in {
        world.access_token = Some(token);
    }
}

#[when(regex = r"^a payment arrives from (x-forwarded-for|forwarded|ip) (\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})$")]
async fn payment_notification(world: &mut TPGWorld, step: &Step, ip_source: String, ip: String) {
    let json = step.docstring().expect("No payment notification");
    let notification = serde_json::from_str::<PaymentNotification>(&json)
        .map_err(|e| error!("{e}"))
        .expect("Failed to parse payment notification");
    trace!("Payment Notification: {notification:?}");
    let (code, body) = world
        .request(Method::POST, "/wallet/incoming_payment", |req| {
            let req = req.json(&notification);
            match ip_source.as_str() {
                "x-forwarded-for" => req.header("x-forwarded-for", ip),
                "forwarded" => req.header("forwarded", format!("for={}", ip)),
                _ => req,
            }
        })
        .await;
    debug!("Got Response: {code} {body}");
    world.response = Some((code, body));
}

#[when(expr = "payment {word} is confirmed")]
async fn confirm_payment(world: &mut TPGWorld, txid: String) {
    let db = world.db.as_ref().expect("No database connection").clone();
    let api = OrderFlowApi::new(db, EventProducers::default());
    let orders = api.confirm_payment(txid).await.expect("Failed to confirm transaction");
    debug!("Paid orders: {}", serde_json::to_string(&orders).expect("Failed to serialize orders"));
}

#[when(regex = r"^a confirmation arrives from (x-forwarded-for|forwarded|ip) (\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})$")]
async fn confirmation_notification(world: &mut TPGWorld, step: &Step, ip_source: String, ip: String) {
    let json = step.docstring().expect("No confirmation notification");
    let confirmation = serde_json::from_str::<TransactionConfirmationNotification>(&json)
        .map_err(|e| error!("{e}"))
        .expect("Failed to parse transaction confirmation");
    trace!("Confirmation: {confirmation:?}");
    let (code, body) = world
        .request(Method::POST, "/wallet/tx_confirmation", |req| {
            let req = req.json(&confirmation);
            match ip_source.as_str() {
                "x-forwarded-for" => req.header("x-forwarded-for", ip),
                "forwarded" => req.header("forwarded", format!("for={}", ip)),
                _ => req,
            }
        })
        .await;
    debug!("Got Response: {code} {body}");
    world.response = Some((code, body));
}

#[then(expr = "I am logged in")]
fn logged_in(world: &mut TPGWorld) {
    assert!(world.logged_in, "Expected to be logged in");
}

#[then(expr = "I am not logged in")]
fn logged_out(world: &mut TPGWorld) {
    assert!(!world.logged_in, "Expected to be logged in");
}

#[then(expr = "my access token starts with {string}")]
async fn access_token_starts_with(world: &mut TPGWorld, prefix: String) {
    let token = world.access_token.as_ref().expect("No access token");
    assert!(token.starts_with(&prefix), "Expected token to start with '{prefix}', got '{token}'");
}

#[when(expr = "{word} {word}s to {string} with body")]
async fn general_request(world: &mut TPGWorld, _user: String, method: String, url: String, step: &Step) {
    world.response = None;
    let method = Method::from_str(method.as_str()).expect("Invalid method");
    let res = world
        .request(method, url.as_str(), |req| match step.docstring().cloned() {
            Some(body) => req.body(body).header("Content-Type", "application/json"),
            None => req,
        })
        .await;
    trace!("Got Response: {} {}", res.0, res.1);
    world.response = Some(res);
}

#[when(expr = "{word} modifies the {word} on the access token to {string}")]
async fn modify_token(world: &mut TPGWorld, _user: String, field: String, value: String) {
    let token = world.access_token.take().expect("No access token");
    debug!("Modifying token: {token}");
    let new_token = match field.as_str() {
        "signature" => modify_signature(token, &value),
        "roles" => modify_roles(token, &value),
        _ => panic!("Invalid field: {field}"),
    };
    debug!("Modified token: {new_token}");
    world.access_token = Some(new_token);
}

#[when(expr = "{word} creates a self-signed SuperAdmin access token")]
async fn create_access_token(world: &mut TPGWorld, user: String) {
    let users = SeedUsers::new();
    let user = users.user(&user);
    let claims = JwtClaims {
        address: user.address.clone(),
        roles: vec![Role::User, Role::ReadAll, Role::Write, Role::SuperAdmin],
    };
    let claims = Claims::new(claims);
    let header = Header::empty().with_token_type("JWT");
    let token = Ristretto256
        .token(&header, &claims, &Ristretto256SigningKey(user.secret.clone()))
        .expect("Failed to sign token");
    world.access_token = Some(token);
}

#[when(expr = "the access token expires")]
async fn expire_access_token(world: &mut TPGWorld) {
    let token = world.access_token.take().expect("No access token");
    let claims = UntrustedToken::new(&token)
        .expect("Invalid token")
        .deserialize_claims_unchecked::<JwtClaims>()
        .expect("Invalid claims");
    let key = world.config.auth.jwt_signing_key.clone();
    let signer = build_jwt_signer(key);
    let token =
        signer.create_signed_token(&claims.custom, std::time::Duration::default()).expect("Failed to sign token");
    sleep(tokio::time::Duration::from_secs(1)).await;
    world.access_token = Some(token);
}

#[when(expr = "Customer #{int} [{string}] places order \"{word}\" for {int} XTR, with memo")]
async fn place_short_order(world: &mut TPGWorld, user: i64, email: String, order_id: String, amount: i64, step: &Step) {
    let now = chrono::Utc::now();
    place_order(world, user, email, order_id, amount, now.to_rfc3339(), step).await;
}

#[when(expr = "Customer #{int} [{string}] places order \"{word}\" for {int} XTR at {string}, with memo")]
async fn place_order(
    world: &mut TPGWorld,
    user: i64,
    email: String,
    order_id: String,
    amount: i64,
    created_at: String,
    step: &Step,
) {
    let memo = step.docstring().map(String::from);
    world.response = None;
    let res = world
        .request(Method::POST, "/shopify/webhook/checkout_create", |req| {
            let mut order = ShopifyOrder::default();
            order.created_at = created_at;
            order.id = order_id;
            order.note = memo;
            order.currency = "XTR".to_string();
            order.total_price = format!("{amount}.00");
            order.user_id = Some(user);
            order.email = Some(email);
            order.customer.id = user;
            let order = serde_json::to_string(&order).expect("Failed to serialize order");
            req.body(order).header("Content-Type", "application/json")
        })
        .await;
    trace!("Got Response: {} {}", res.0, res.1);
    world.response = Some(res);
}

#[then(expr = "Customer #{int} has current orders worth {int} XTR")]
async fn check_current_orders(world: &mut TPGWorld, account_id: i64, total: i64) {
    let db = world.db.as_ref().expect("No database connection");
    let account = db.fetch_user_account(account_id).await.expect("Failed to fetch account").expect("No account found");
    trace!("User account: {account:?}");
    let expected_current_orders = MicroTari::from_tari(total);
    assert_eq!(account.current_orders, expected_current_orders);
}

#[then(regex = r"^account for (\w+) has (total|current) orders worth (-?\d+) XTR")]
async fn account_orders(world: &mut TPGWorld, cust_id: String, total_type: String, total: i64) {
    let db = world.db.as_ref().expect("No database connection");
    let account = db
        .fetch_user_account_for_customer_id(&cust_id)
        .await
        .expect("Failed to fetch account")
        .expect("No account found");
    trace!("User account: {account:?}");
    let expected_total = match total_type.as_str() {
        "total" => account.total_orders,
        "current" => account.current_orders,
        _ => panic!("Invalid total type: {total_type}"),
    };
    assert_eq!(expected_total, MicroTari::from_tari(total));
}

#[then(expr = "order \"{word}\" is in state {word}")]
async fn check_order_state(world: &mut TPGWorld, order_id: String, state: String) {
    let db = world.db.as_ref().expect("No database connection");
    let oid = OrderId::from(order_id);
    let order = db.fetch_order_by_order_id(&oid).await.expect("Failed to fetch order").expect("No order found");
    let status = state.parse().expect("Invalid order status");
    assert_eq!(order.status, status);
}

#[then(regex = r#"^(\w+) has a (current|pending) balance of (\d+) Tari$"#)]
async fn check_balance(world: &mut TPGWorld, user: String, bal_type: String, balance: i64) {
    let db = world.db.as_ref().expect("No database connection");
    let users = SeedUsers::new();
    let user = users.user(&user);
    let account = db
        .fetch_user_account_for_address(&user.address)
        .await
        .expect("Failed to fetch account")
        .expect("No account found");
    let expected_balance = MicroTari::from_tari(balance);
    let actual_balance = match bal_type.as_str() {
        "current" => account.current_balance,
        "pending" => account.current_pending,
        _ => panic!("Invalid balance type: {bal_type}"),
    };
    assert_eq!(actual_balance, expected_balance);
}

#[then(regex = r#"^account for customer (\w+) has a (current|pending) balance of (-?\d+) Tari$"#)]
async fn check_customer_balance(world: &mut TPGWorld, cust_id: String, bal_type: String, balance: i64) {
    let db = world.db.as_ref().expect("No database connection");
    let account = db
        .fetch_user_account_for_customer_id(&cust_id)
        .await
        .expect("Failed to fetch account")
        .expect("No account found");
    let expected_balance = MicroTari::from_tari(balance);
    let actual_balance = match bal_type.as_str() {
        "current" => account.current_balance,
        "pending" => account.current_pending,
        _ => panic!("Invalid balance type: {bal_type}"),
    };
    assert_eq!(actual_balance, expected_balance);
}

#[then(expr = "the {word} trigger fires with")]
async fn check_event_trigger(world: &mut TPGWorld, event_type: String, step: &Step) {
    let last_event = world.last_event(&event_type);
    if last_event.is_none() {
        panic!("Expected {event_type} event but no event was triggered");
    }
    let ev = last_event.unwrap();
    let json = match ev {
        EventType::NewOrder(e) => serde_json::to_string(&e),
        EventType::OrderPaid(e) => serde_json::to_string(&e),
        EventType::OrderAnnulled(e) => serde_json::to_string(&e),
        EventType::OrderModified(e) => serde_json::to_string(&e),
        EventType::PaymentReceived(e) => serde_json::to_string(&e),
        EventType::Confirmation(e) => serde_json::to_string(&e),
        EventType::OrderClaimed(e) => serde_json::to_string(&e),
    }
    .expect("Failed to serialize event");
    let expected = step.docstring().expect("No expected OrderModifiedEvent in docstring");
    assert!(json_is_subset_of(expected, &json), "Expected order to be '{expected}', got '{json}'");
}

#[then(expr = "address {word} has roles {string}")]
async fn check_roles(world: &mut TPGWorld, address: String, roles: String) {
    let db = world.db.as_ref().expect("No database connection");
    let roles = extract_roles(&roles);
    let address = address.parse().expect("Invalid address");
    let account =
        db.check_address_has_roles(&address, &roles).await.map_err(|e| error!("Failed to fetch account. {e}"));
    assert!(account.is_ok())
}

#[given(expr = "a payment of {int} XTR from address {word} in tx {word}")]
async fn instapay(world: &mut TPGWorld, amount: i64, sender: TariAddress, txid: String) {
    let amount = MicroTari::from_tari(amount);
    let db = world.db.as_ref().expect("No database connection");
    let payment = NewPayment::new(sender, amount, txid);
    let (acc, payment) = db.process_new_payment_for_pubkey(payment).await.expect("Failed to process payment");
    info!("Processed payment for account {acc}: {payment:?}");
    confirm_payment(world, payment.txid).await;
}

#[when(expr = "I expire old orders")]
async fn expire_old_orders(world: &mut TPGWorld) {
    let db = world.db.as_ref().expect("No database connection");
    let unclaimed_limit = Duration::seconds(2);
    let unpaid_limit = Duration::seconds(4);
    let orders = db.expire_old_orders(unclaimed_limit, unpaid_limit).await.expect("Failed to expire orders");
    info!("Expired orders: {}", serde_json::to_string(&orders).expect("Failed to serialize orders"));
}

fn modify_signature(token: String, value: &str) -> String {
    let mut parts = token.split('.').map(|s| s.to_owned()).collect::<Vec<_>>();
    let n = value.len();
    let sig = parts.iter_mut().nth(2).expect("No signature");
    sig.replace_range(0..n, value);
    format!("{}.{}.{}", parts[0], parts[1], parts[2])
}

fn modify_roles(orig_token: String, roles: &str) -> String {
    let token = UntrustedToken::new(&orig_token).expect("Invalid token");
    let mut claims = token.deserialize_claims_unchecked::<JwtClaims>().expect("Invalid claims");
    claims.custom.roles = extract_roles(roles);
    let new_token = (Ristretto256 {})
        .token(token.header(), &claims, &Ristretto256SigningKey::default())
        .expect("Failed to sign token");
    let new_parts = new_token.split('.').collect::<Vec<_>>();
    let orig_parts = orig_token.split('.').collect::<Vec<_>>();
    format!("{}.{}.{}", orig_parts[0], new_parts[1], orig_parts[2])
}

fn extract_token(docstring: Option<&String>) -> (String, String) {
    docstring
        .map(|s| {
            let s = s.replace("\\\n", "");
            let mut iter = s.split(':').take(2).map(|s| s.trim().to_string());
            (iter.next().unwrap_or_else(|| "foo".into()), iter.next().unwrap_or_default())
        })
        .unwrap_or_else(|| ("none".into(), String::default()))
}

fn extract_roles(roles: &str) -> Vec<Role> {
    roles.split(',').map(|s| s.trim()).map(|r| r.parse::<Role>().expect("Invalid role")).collect()
}
