use actix_web::{http::StatusCode, web, web::ServiceConfig};
use chrono::{Days, TimeZone, Utc};
use log::debug;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::{Order, OrderId, OrderStatusType, Role, UserAccount},
    AccountApi,
};
use tpg_common::MicroTari;

use super::helpers::{get_request, issue_token};
use crate::{
    auth::JwtClaims,
    endpoint_tests::mocks::MockAccountManager,
    routes::{MyOrdersRoute, OrdersRoute},
};

#[actix_web::test]
async fn fetch_my_orders_no_headers() {
    let _ = env_logger::try_init().ok();
    let err = get_request("", "/orders", configure).await.expect_err("Expected error");
    assert_eq!(
        err,
        "An error occurred, no cookie containing a jwt was found in the request. Please first authenticate with this \
         application."
    );
}

#[actix_web::test]
async fn fetch_my_orders() {
    let _ = env_logger::try_init().ok();
    let token = valid_token(vec![Role::User]);
    let (status, body) = get_request(&token, "/orders", configure).await.expect("Request failed");
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, ORDERS_JSON);
}

#[actix_web::test]
async fn fetch_my_orders_invalid_sig() {
    let _ = env_logger::try_init().ok();
    let mut token = valid_token(vec![Role::User]);
    token.replace_range(token.len() - 10..token.len() - 5, "00000");
    debug!("Calling /orders with invalid token {token}");
    let err = get_request(&token, "/orders", configure).await.expect_err("Expected error");
    assert_eq!(err, "An error occurred validating the jwt.\n\t Error: \"signature has failed verification\"");
}

#[actix_web::test]
async fn try_fetch_another_users_orders_as_admin() {
    let _ = env_logger::try_init().ok();
    let token = valid_token(vec![Role::ReadAll]);
    let (status, body) =
        get_request(&token, "/orders/b4db54f75421a02b0d0056fb7203df23c742b25e41283976bdaa7fe63de1ad234d", configure)
            .await
            .expect("Request failed");
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, ORDERS_JSON);
}

#[actix_web::test]
async fn try_fetch_another_users_orders_as_normal_user() {
    let _ = env_logger::try_init().ok();
    let token = valid_token(vec![Role::User]);
    let err =
        get_request(&token, "/orders/fc899cd4395e86e9409fc892f5b0a064373a4300321650e205e446374f6b8f073d", configure)
            .await
            .expect_err("Request should have failed");
    assert_eq!(err, "Insufficient permissions.");
}

fn valid_token(roles: Vec<Role>) -> String {
    issue_token(
        JwtClaims {
            address: TariAddress::from_hex("b4db54f75421a02b0d0056fb7203df23c742b25e41283976bdaa7fe63de1ad234d")
                .unwrap(),
            roles,
        },
        Utc::now() + Days::new(1),
    )
}

fn configure(cfg: &mut ServiceConfig) {
    let mut account_manager = MockAccountManager::new();
    account_manager.expect_fetch_orders_for_account().returning(move |_| Ok(orders_response()));
    account_manager
        .expect_fetch_user_account_for_address()
        .returning(|_| Ok(Some(UserAccount { id: 1, ..UserAccount::default() })));
    let accounts_api = AccountApi::new(account_manager);
    cfg.service(MyOrdersRoute::<MockAccountManager>::new())
        .service(OrdersRoute::<MockAccountManager>::new())
        .app_data(web::Data::new(accounts_api));
}

// Mock response to `fetch_orders_for_account` call
fn orders_response() -> Vec<Order> {
    vec![
        Order {
            id: 0,
            order_id: OrderId("0000001".into()),
            customer_id: "1".to_string(),
            memo: None,
            total_price: MicroTari::from(100),
            original_price: None,
            currency: "XTR".to_string(),
            created_at: Utc.with_ymd_and_hms(2024, 2, 29, 13, 30, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2024, 2, 29, 13, 30, 0).unwrap(),
            status: OrderStatusType::Paid,
        },
        Order {
            id: 1,
            order_id: OrderId("0000002".into()),
            customer_id: "1".to_string(),
            memo: None,
            total_price: MicroTari::from(150),
            original_price: None,
            currency: "XTR".to_string(),
            created_at: Utc.with_ymd_and_hms(2024, 3, 15, 18, 30, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2024, 3, 16, 11, 20, 0).unwrap(),
            status: OrderStatusType::Cancelled,
        },
    ]
}

const ORDERS_JSON: &str = r#"{"address":"b4db54f75421a02b0d0056fb7203df23c742b25e41283976bdaa7fe63de1ad234d","total_orders":0,"orders":[{"id":0,"order_id":"0000001","customer_id":"1","memo":null,"total_price":100,"original_price":null,"currency":"XTR","created_at":"2024-02-29T13:30:00Z","updated_at":"2024-02-29T13:30:00Z","status":"Paid"},{"id":1,"order_id":"0000002","customer_id":"1","memo":null,"total_price":150,"original_price":null,"currency":"XTR","created_at":"2024-03-15T18:30:00Z","updated_at":"2024-03-16T11:20:00Z","status":"Cancelled"}]}"#;
