use actix_web::{body::MessageBody, http::StatusCode, test, test::TestRequest, web, web::ServiceConfig, App};
use chrono::Utc;
use log::*;
use tari_jwt::{
    jwt_compact::{AlgorithmExt, Claims, UntrustedToken},
    Ristretto256,
    Ristretto256VerifyingKey,
};
use tari_payment_engine::{db_types::Role, traits::AuthApiError, AuthApi};

use super::mocks::*;
use crate::{
    auth::{JwtClaims, TokenIssuer},
    config::AuthConfig,
    routes::AuthRoute,
};

#[actix_web::test]
async fn login_without_headers() {
    let _ = env_logger::try_init().ok();
    let req = TestRequest::post().uri("/auth").to_request();
    let config = AuthConfig::default();
    let func = configure_app(config.clone(), Ok(()));
    let app = App::new().configure(func);
    let app = test::init_service(app).await;
    let (_req, res) = test::call_service(&app, req).await.into_parts();
    let status = res.status();
    let body = String::from_utf8_lossy(&res.into_body().try_into_bytes().unwrap()).into_owned();
    info!("Response body: {body}");
    assert!(body.contains("Auth token signature invalid or not provided"), "was: {body}");
    assert!(status.is_client_error())
}

#[actix_web::test]
async fn login_with_invalid_header() {
    let _ = env_logger::try_init().ok();
    let (status, body, _) = post_request("made up nonsense", Ok(())).await;
    assert!(body.contains("Authentication Error. Login token is not in the correct format."), "was: {body}");
    assert_eq!(status.as_u16(), StatusCode::BAD_REQUEST.as_u16());
}

#[actix_web::test]
async fn login_with_invalid_signature() {
    // Valid token, but invalid signature
    let token = "eyJhbGciOiJSaXN0cmV0dG8yNTYifQ.\
        eyJhZGRyZXNzIjp7Im5ldHdvcmsiOiJuZXh0bmV0IiwicHVibGljX2tleSI6IjEyYTI1MDRhNzhmMDg5MzBjMmQzMzU3MDhmYWU4MDY5NmIyMTdkMjNiZDJkNDczZTEyN2Q4ZjVhMzBlMjgxNjUifSwibm9uY2UiOjE3MTE0NDUxMTgsImRlc2lyZWRfcm9sZXMiOlsidXNlciIsIndyaXRlIl19.\
        bad_sig_Uip03HFi5q65zE-QBq8iyEuT-IkLy9KeSHmB3UGkPIJXSDrKDVU_lg6JfBY4ch7BxwyH5iLDEiDzAQ";
    let (status, body, _) = post_request(token, Ok(())).await;
    assert!(body.contains("Authentication Error. Login token signature is invalid."));
    assert_eq!(status.as_u16(), StatusCode::UNAUTHORIZED.as_u16());
}

#[actix_web::test]
async fn login_with_valid_token() {
    let token = "eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ.eyJhZGRyZXNzIjp7Im5ldHdvcmsiOiJtYWlubmV0IiwicHVibGljX2tleSI6IjEyYTI1MDRhNzhmMDg5MzBjMmQzMzU3MDhmYWU4MDY5NmIyMTdkMjNiZDJkNDczZTEyN2Q4ZjVhMzBlMjgxNjUifSwibm9uY2UiOjE3MTE0NDg2NDksImRlc2lyZWRfcm9sZXMiOlsidXNlciIsIndyaXRlIl19.gm2Z6FxNyMmLT_dcEGu_iH9_wBm029OY_eqw__hZ0yXpa0ccVeBbF1lTfYU5xEhmGtQXtwhOjC8l2SUm9QB8CQ";
    let (status, s, config) = post_request(token, Ok(())).await;
    let token = validate_token(&s, &config.jwt_verification_key).unwrap();
    assert!(status.is_success());
    assert_eq!(token.address.to_hex(), "12a2504a78f08930c2d335708fae80696b217d23bd2d473e127d8f5a30e28165de");
    assert_eq!(&token.roles, &[Role::User, Role::Write]);
}

#[actix_web::test]
async fn login_with_disallowed_roles() {
    let token = "eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ.eyJhZGRyZXNzIjp7Im5ldHdvcmsiOiJtYWlubmV0IiwicHVibGljX2tleSI6IjEyYTI1MDRhNzhmMDg5MzBjMmQzMzU3MDhmYWU4MDY5NmIyMTdkMjNiZDJkNDczZTEyN2Q4ZjVhMzBlMjgxNjUifSwibm9uY2UiOjE3MTE0NDg2NDksImRlc2lyZWRfcm9sZXMiOlsidXNlciIsIndyaXRlIl19.gm2Z6FxNyMmLT_dcEGu_iH9_wBm029OY_eqw__hZ0yXpa0ccVeBbF1lTfYU5xEhmGtQXtwhOjC8l2SUm9QB8CQ";
    let (status, body, _) = post_request(token, Err(AuthApiError::RoleNotAllowed(4))).await;
    assert_eq!(status.as_u16(), StatusCode::FORBIDDEN.as_u16());
    assert_eq!(
        body,
        r#"{"error":"Authentication Error. Insufficient Permissions. User requested at least 4 roles that are not allowed"}"#
    );
}

#[actix_web::test]
async fn login_with_no_preexisting_user_account() {
    let token = "eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ.eyJhZGRyZXNzIjp7Im5ldHdvcmsiOiJtYWlubmV0IiwicHVibGljX2tleSI6IjEyYTI1MDRhNzhmMDg5MzBjMmQzMzU3MDhmYWU4MDY5NmIyMTdkMjNiZDJkNDczZTEyN2Q4ZjVhMzBlMjgxNjUifSwibm9uY2UiOjE3MTE0NDg2NDksImRlc2lyZWRfcm9sZXMiOlsidXNlciIsIndyaXRlIl19.gm2Z6FxNyMmLT_dcEGu_iH9_wBm029OY_eqw__hZ0yXpa0ccVeBbF1lTfYU5xEhmGtQXtwhOjC8l2SUm9QB8CQ";
    let (status, body, _) = post_request(token, Err(AuthApiError::AddressNotFound)).await;
    assert_eq!(status.as_u16(), StatusCode::FORBIDDEN.as_u16());
    assert_eq!(body, r#"{"error":"Authentication Error. User account not found."}"#);
}

#[actix_web::test]
async fn login_with_invalid_nonce() {
    let token = "eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ.eyJhZGRyZXNzIjp7Im5ldHdvcmsiOiJtYWlubmV0IiwicHVibGljX2tleSI6IjEyYTI1MDRhNzhmMDg5MzBjMmQzMzU3MDhmYWU4MDY5NmIyMTdkMjNiZDJkNDczZTEyN2Q4ZjVhMzBlMjgxNjUifSwibm9uY2UiOjE3MTE0NDg2NDksImRlc2lyZWRfcm9sZXMiOlsidXNlciIsIndyaXRlIl19.gm2Z6FxNyMmLT_dcEGu_iH9_wBm029OY_eqw__hZ0yXpa0ccVeBbF1lTfYU5xEhmGtQXtwhOjC8l2SUm9QB8CQ";
    let (status, body, _) = post_request(token, Err(AuthApiError::InvalidNonce)).await;
    assert_eq!(status.as_u16(), StatusCode::UNAUTHORIZED.as_u16());
    assert_eq!(
        body,
        r#"{"error":"Authentication Error. Login token signature is invalid. Nonce is not strictly increasing."}"#
    );
}

fn configure_app(config: AuthConfig, update_nonce_result: Result<(), AuthApiError>) -> impl FnOnce(&mut ServiceConfig) {
    move |cfg| {
        let mut auth_manager = MockAuthManager::new();
        auth_manager.expect_update_nonce_for_address().return_const(update_nonce_result);
        auth_manager.expect_check_auth_account_exists().returning(move |_| Ok(true));
        auth_manager.expect_check_address_has_roles().returning(|_a, _b| Ok(()));
        let auth_api = AuthApi::new(auth_manager);
        let jwt_signer = TokenIssuer::new(&config.clone());
        cfg.app_data(web::Data::new(auth_api))
            .app_data(web::Data::new(jwt_signer))
            .service(AuthRoute::<MockAuthManager>::new());
    }
}

async fn post_request(
    auth_header: &str,
    update_nonce_result: Result<(), AuthApiError>,
) -> (StatusCode, String, AuthConfig) {
    let req = TestRequest::post().uri("/auth").insert_header(("tpg_auth_token", auth_header)).to_request();
    let config = AuthConfig::default();
    let app = App::new().configure(configure_app(config.clone(), update_nonce_result));
    let app = test::init_service(app).await;
    debug!("Making request");
    let (_, res) = test::call_service(&app, req).await.into_parts();
    let status = res.status();
    let body = String::from_utf8_lossy(&res.into_body().try_into_bytes().unwrap()).into_owned();
    (status, body, config)
}
fn validate_token(token: &str, verifying_key: &Ristretto256VerifyingKey) -> Result<JwtClaims, String> {
    debug!("Validating token: {token}");
    let untrusted_token = UntrustedToken::new(token).map_err(|e| format!("Invalid token format: {e:?}"))?;
    let _claims: Claims<JwtClaims> =
        untrusted_token.deserialize_claims_unchecked().map_err(|e| format!("Claims validation error: {e:?}"))?;
    let (header, claims) = Ristretto256
        .validator(verifying_key)
        .validate(&untrusted_token)
        .map_err(|e| format!("Signature error: {e}"))?
        .into_parts();
    debug!("Login token validated successfully. Header: {header:?}. Claims: {claims:?}");
    let expiry = claims.expiration.unwrap().signed_duration_since(Utc::now());
    assert!(expiry.num_hours() < 24 && expiry.num_hours() >= 23, "Expiry: {}", expiry.num_hours());
    assert_eq!(header.token_type.unwrap(), "JWT");
    Ok(claims.custom)
}
