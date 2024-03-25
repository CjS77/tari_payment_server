use crate::auth::{JwtClaims, TokenIssuer};
use actix_web::http::StatusCode;
use actix_web::web::ServiceConfig;
use actix_web::HttpResponse;
use tari_jwt::jwt_compact::{AlgorithmExt, Claims, UntrustedToken};
use tari_jwt::{Ristretto256, Ristretto256VerifyingKey};
use tari_payment_engine::db_types::Role;
use tari_payment_engine::AuthApi;

use super::mocks::*;
use crate::routes::AuthRoute;

use crate::config::AuthConfig;
use actix_web::body::MessageBody;
use actix_web::test::TestRequest;
use actix_web::{test, web, App};
use log::*;

#[actix_web::test]
async fn login_without_headers() {
    let _ = env_logger::try_init().ok();
    let req = TestRequest::post().uri("/auth").to_request();
    let config = AuthConfig::default();
    let func = configure_app(config.clone(), None);
    let app = App::new().configure(func);
    let app = test::init_service(app).await;
    let (_req, res) = test::call_service(&app, req).await.into_parts();
    let status = res.status();
    let body = String::from_utf8_lossy(&res.into_body().try_into_bytes().unwrap()).into_owned();
    info!("Response body: {body}");
    assert!(
        body.contains("Auth token signature invalid or not provided"),
        "was: {body}"
    );
    assert!(status.is_client_error())
}

#[actix_web::test]
async fn login_with_headers() {
    let _ = env_logger::try_init().ok();
    let req = TestRequest::post()
        .uri("/auth")
        .insert_header(("Authorization", "made-up nonsense"))
        .to_request();
    debug!("Building app");
    let config = AuthConfig::default();
    let app = App::new().configure(configure_app(config.clone(), Some(42)));
    let app = test::init_service(app).await;
    debug!("Making request");
    let (_, res) = test::call_service(&app, req).await.into_parts();
    let status = res.status();
    let body = String::from_utf8_lossy(&res.into_body().try_into_bytes().unwrap()).into_owned();
    info!("Response body: {body}");
    assert!(
        body.contains("Authentication Error. Login token is not in the correct format."),
        "was: {body}"
    );
    assert_eq!(status.as_u16(), StatusCode::BAD_REQUEST.as_u16());

    // Valid token, but invalid signature
    let token = "eyJhbGciOiJSaXN0cmV0dG8yNTYifQ.\
        eyJhZGRyZXNzIjp7Im5ldHdvcmsiOiJuZXh0bmV0IiwicHVibGljX2tleSI6IjEyYTI1MDRhNzhmMDg5MzBjMmQzMzU3MDhmYWU4MDY5NmIyMTdkMjNiZDJkNDczZTEyN2Q4ZjVhMzBlMjgxNjUifSwibm9uY2UiOjE3MTE0NDUxMTgsImRlc2lyZWRfcm9sZXMiOlsidXNlciIsIndyaXRlIl19.\
        bad_sig_Uip03HFi5q65zE-QBq8iyEuT-IkLy9KeSHmB3UGkPIJXSDrKDVU_lg6JfBY4ch7BxwyH5iLDEiDzAQ";
    let req = TestRequest::post()
        .uri("/auth")
        .insert_header(("Authorization", token))
        .to_request();
    let (_, res) = test::call_service(&app, req).await.into_parts();
    let status = res.status();
    let body = String::from_utf8_lossy(&res.into_body().try_into_bytes().unwrap()).into_owned();
    info!("Response body: {body}");
    assert!(body.contains("Authentication Error. Login token signature is invalid."));
    assert_eq!(status.as_u16(), StatusCode::UNAUTHORIZED.as_u16());

    // Valid token
    let token = "eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ.eyJhZGRyZXNzIjp7Im5ldHdvcmsiOiJtYWlubmV0IiwicHVibGljX2tleSI6IjEyYTI1MDRhNzhmMDg5MzBjMmQzMzU3MDhmYWU4MDY5NmIyMTdkMjNiZDJkNDczZTEyN2Q4ZjVhMzBlMjgxNjUifSwibm9uY2UiOjE3MTE0NDg2NDksImRlc2lyZWRfcm9sZXMiOlsidXNlciIsIndyaXRlIl19.gm2Z6FxNyMmLT_dcEGu_iH9_wBm029OY_eqw__hZ0yXpa0ccVeBbF1lTfYU5xEhmGtQXtwhOjC8l2SUm9QB8CQ";
    let req = TestRequest::post()
        .uri("/auth")
        .insert_header(("Authorization", token))
        .to_request();
    let (_, res) = test::call_service(&app, req).await.into_parts();
    let status = res.status();
    let s = response_to_string(res);
    println!("Response body: {s}");
    let token = validate_token(&s, &config.jwt_verification_key).unwrap();
    assert!(status.is_success());
    assert_eq!(
        token.address.to_hex(),
        "12a2504a78f08930c2d335708fae80696b217d23bd2d473e127d8f5a30e28165de"
    );
    assert_eq!(&token.roles, &[Role::User, Role::Write]);
    assert_eq!(token.cust_id, Some(42));
}

fn configure_app(config: AuthConfig, cust_id: Option<i64>) -> impl FnOnce(&mut ServiceConfig) {
    move |cfg| {
        let mut auth_manager = MockAuthManager::new();
        auth_manager
            .expect_update_nonce_for_address()
            .returning(move |_, _| Ok(cust_id.clone()));
        let auth_api = AuthApi::new(auth_manager);
        let jwt_signer = TokenIssuer::new(&config.clone());
        cfg.app_data(web::Data::new(auth_api))
            .app_data(web::Data::new(jwt_signer))
            .service(AuthRoute::<MockAuthManager>::new());
    }
}

fn response_to_string(res: HttpResponse) -> String {
    let body = res.into_body().try_into_bytes().unwrap();
    String::from_utf8_lossy(&body).into_owned()
}

fn validate_token(
    token: &str,
    verifying_key: &Ristretto256VerifyingKey,
) -> Result<JwtClaims, String> {
    let untrusted_token =
        UntrustedToken::new(token).map_err(|e| format!("Invalid token format: {e:?}"))?;
    let _claims: Claims<JwtClaims> = untrusted_token
        .deserialize_claims_unchecked()
        .map_err(|e| format!("Claims validation error: {e:?}"))?;
    let (header, claims) = Ristretto256
        .validator(verifying_key)
        .validate(&untrusted_token)
        .map_err(|e| format!("Signature error: {e}"))?
        .into_parts();
    debug!("Login token validated successfully. Header: {header:?}. Claims: {claims:?}");
    Ok(claims.custom)
}
