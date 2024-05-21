use std::str::FromStr;

use cucumber::{gherkin::Step, then, when};
use e2e::helpers::json_is_subset_of;
use log::*;
use reqwest::Method;
use tari_jwt::{
    jwt_compact::{AlgorithmExt, Claims, Header, UntrustedToken},
    Ristretto256,
    Ristretto256SigningKey,
};
use tari_payment_engine::db_types::Role;
use tari_payment_server::auth::{build_jwt_signer, JwtClaims};
use tokio::time::sleep;

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
