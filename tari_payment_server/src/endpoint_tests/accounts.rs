use actix_web::{http::StatusCode, web, web::ServiceConfig};
use chrono::{Days, TimeZone, Utc};
use log::debug;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::{MicroTari, Role, UserAccount},
    AccountApi,
};

use super::helpers::{get_request, issue_token};
use crate::{
    auth::JwtClaims,
    endpoint_tests::mocks::MockAccountManager,
    routes::{AccountRoute, MyAccountRoute},
};

#[actix_web::test]
async fn fetch_my_account_no_headers() {
    let _ = env_logger::try_init().ok();
    let err = get_request("", "/account", configure).await.expect_err("Expected error");
    assert_eq!(
        err,
        "An error occurred, no cookie containing a jwt was found in the request. Please first authenticate with this \
         application."
    );
}

#[actix_web::test]
async fn fetch_my_account_expired_token() {
    let claims = JwtClaims {
        address: TariAddress::from_hex("b4db54f75421a02b0d0056fb7203df23c742b25e41283976bdaa7fe63de1ad234d").unwrap(),
        roles: vec![Role::User],
    };
    let expired = Utc::now() - Days::new(1);
    debug!("Calling /account with expired token {claims:?}");
    let token = issue_token(claims, expired);
    let err = get_request(&token, "/account", configure).await.expect_err("Expected error");
    assert_eq!(err, "An error occurred validating the jwt.\n\t Error: \"token has expired\"");
}

#[actix_web::test]
async fn fetch_my_account_valid_token() {
    let claims = JwtClaims {
        address: TariAddress::from_hex("b4db54f75421a02b0d0056fb7203df23c742b25e41283976bdaa7fe63de1ad234d").unwrap(),
        roles: vec![Role::User],
    };
    let token = issue_token(claims, Utc::now() + Days::new(1));
    let (status, body) = get_request(&token, "/account", configure).await.expect("Failed to make request");
    assert_eq!(status, StatusCode::OK);
    let json = r#"
    {"id":1,"created_at":"2024-03-01T10:30:00Z","updated_at":"2024-03-01T10:30:00Z","total_received":1000000,"current_pending":0,"current_balance":1000000,"total_orders":0,"current_orders":0}
    "#;
    assert_eq!(body, json.trim());
}

#[actix_web::test]
async fn fetch_account_from_admin() {
    let claims = JwtClaims {
        address: TariAddress::from_hex("b4db54f75421a02b0d0056fb7203df23c742b25e41283976bdaa7fe63de1ad234d").unwrap(),
        roles: vec![Role::ReadAll],
    };
    let token = issue_token(claims, Utc::now() + Days::new(1));
    let (status, body) =
        get_request(&token, "/account/fc899cd4395e86e9409fc892f5b0a064373a4300321650e205e446374f6b8f073d", configure)
            .await
            .expect("Failed to make request");
    assert_eq!(status, StatusCode::OK);
    let json = r#"
    {"id":1,"created_at":"2024-03-01T10:30:00Z","updated_at":"2024-03-01T10:30:00Z","total_received":1000000,"current_pending":0,"current_balance":1000000,"total_orders":0,"current_orders":0}
    "#;
    assert_eq!(body, json.trim());
}

#[actix_web::test]
async fn fetch_account_from_user() {
    let claims = JwtClaims {
        address: TariAddress::from_hex("b4db54f75421a02b0d0056fb7203df23c742b25e41283976bdaa7fe63de1ad234d").unwrap(),
        roles: vec![Role::User],
    };
    let token = issue_token(claims, Utc::now() + Days::new(1));
    let err =
        get_request(&token, "/account/fc899cd4395e86e9409fc892f5b0a064373a4300321650e205e446374f6b8f073d", configure)
            .await
            .expect_err("Request should have failed");
    assert_eq!(err, "Insufficient permissions.");
}

#[actix_web::test]
async fn fetch_account_from_users_own_address() {
    let claims = JwtClaims {
        address: TariAddress::from_hex("b4db54f75421a02b0d0056fb7203df23c742b25e41283976bdaa7fe63de1ad234d").unwrap(),
        roles: vec![Role::User],
    };
    let token = issue_token(claims, Utc::now() + Days::new(1));
    let (status, body) = get_request(&token, "/account", configure).await.expect("Request should have succeeded");
    assert_eq!(status, StatusCode::OK);
    let json = r#"
    {"id":1,"created_at":"2024-03-01T10:30:00Z","updated_at":"2024-03-01T10:30:00Z","total_received":1000000,"current_pending":0,"current_balance":1000000,"total_orders":0,"current_orders":0}
    "#;
    assert_eq!(body, json.trim());
}

fn configure(cfg: &mut ServiceConfig) {
    let account = UserAccount {
        id: 1,
        created_at: Utc.with_ymd_and_hms(2024, 3, 1, 10, 30, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2024, 3, 1, 10, 30, 0).unwrap(),
        total_received: MicroTari::from(1_000_000),
        current_pending: Default::default(),
        current_balance: MicroTari::from(1_000_000),
        current_orders: Default::default(),
        total_orders: Default::default(),
    };
    let mut account_manager = MockAccountManager::new();
    account_manager.expect_fetch_user_account_for_address().returning(move |_| Ok(Some(account.clone())));
    let accounts_api = AccountApi::new(account_manager);
    cfg.service(MyAccountRoute::<MockAccountManager>::new())
        .service(AccountRoute::<MockAccountManager>::new())
        .app_data(web::Data::new(accounts_api));
}
