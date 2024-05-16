use std::str::FromStr;

use cucumber::{gherkin::Step, then, when};
use log::debug;
use reqwest::Method;
use tari_payment_engine::db_types::Role;

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
    let expected = step.docstring().expect("No expected response");
    assert!(json_is_subset_of(res_msg.as_str(), expected), "Expected response to be '{expected}', got '{res_msg}'");
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
    let method = Method::from_str(method.as_str()).expect("Invalid method");
    let res = world
        .request(method, url.as_str(), |req| match step.docstring().cloned() {
            Some(body) => req.body(body).header("Content-Type", "application/json"),
            None => req,
        })
        .await;
    world.response = Some(res);
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

fn json_is_subset_of(part: &str, complete: &str) -> bool {
    let part: serde_json::Value = serde_json::from_str(part).expect("Invalid JSON");
    let complete: serde_json::Value = serde_json::from_str(complete).expect("Invalid JSON");
    for (key, value) in part.as_object().expect("Not an object") {
        if let Some(complete_value) = complete.get(key) {
            if complete_value != value {
                return false;
            }
        }
    }
    true
}
