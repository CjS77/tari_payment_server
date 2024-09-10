use actix_web::{web, web::ServiceConfig};
use chrono::{Days, Utc};
use log::debug;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{db_types::Role, AccountApi};

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
        address: TariAddress::from_base58("14AYt2hhhn4VydAXNJ6i7ZfRNZGoGSp713dHjMYCoK5hYw2").unwrap(),
        roles: vec![Role::User],
    };
    let expired = Utc::now() - Days::new(1);
    debug!("Calling /account with expired token {claims:?}");
    let token = issue_token(claims, expired);
    let err = get_request(&token, "/account", configure).await.expect_err("Expected error");
    assert_eq!(err, "An error occurred validating the jwt.\n\t Error: \"token has expired\"");
}

fn configure(cfg: &mut ServiceConfig) {
    let account_manager = MockAccountManager::new();
    let accounts_api = AccountApi::new(account_manager);
    cfg.service(MyAccountRoute::<MockAccountManager>::new())
        .service(AccountRoute::<MockAccountManager>::new())
        .app_data(web::Data::new(accounts_api));
}
