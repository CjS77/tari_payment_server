use actix_jwt_auth_middleware::AuthenticationService;
use actix_web::{body::MessageBody, http::StatusCode, test, test::TestRequest, web::ServiceConfig, App};
use chrono::{DateTime, Utc};
use log::debug;
use tari_jwt::{
    jwt_compact::{AlgorithmExt, Claims},
    tari_crypto::{
        ristretto::{RistrettoPublicKey, RistrettoSecretKey},
        tari_utilities::hex::Hex,
    },
    Ristretto256,
    Ristretto256SigningKey,
    Ristretto256VerifyingKey,
};

use crate::{
    auth::{build_tps_authority, JwtClaims},
    config::AuthConfig,
};

// Creates a test `AuthConfig` for issuing tokens. DO NOT re-use these keys anywhere.
pub fn get_auth_config() -> AuthConfig {
    AuthConfig {
        jwt_signing_key: Ristretto256SigningKey(
            RistrettoSecretKey::from_hex("925842e11914fdd0c9a2ab8a38dac9de57b3e392372cde1661b1a84b1d8e430e").unwrap(),
        ),
        jwt_verification_key: Ristretto256VerifyingKey(
            RistrettoPublicKey::from_hex("b4db54f75421a02b0d0056fb7203df23c742b25e41283976bdaa7fe63de1ad23").unwrap(),
        ),
    }
}

pub fn issue_token(claims: JwtClaims, expiry: DateTime<Utc>) -> String {
    let config = get_auth_config();
    let header = tari_jwt::jwt_compact::Header::empty().with_token_type("JWT");
    let signer = Ristretto256 {};
    let mut claims = Claims::<JwtClaims>::new(claims);
    claims.expiration = Some(expiry);
    signer.token(&header, &claims, &config.jwt_signing_key).expect("Failed to sign token")
}

pub async fn get_request(
    auth_header: &str,
    path: &str,
    configure: fn(&mut ServiceConfig),
) -> Result<(StatusCode, String), String> {
    let mut req = TestRequest::get().uri(path);
    if !auth_header.is_empty() {
        req = req.insert_header(("tpg_access_token", auth_header));
    }
    let req = req.to_request();
    let config = get_auth_config();
    let authority = build_tps_authority(config.clone());
    let app = App::new().wrap(AuthenticationService::new(authority)).configure(configure);

    let service = test::init_service(app).await;
    debug!("Making request");
    let (_, res) = test::try_call_service(&service, req).await.map_err(|e| e.to_string())?.into_parts();
    let status = res.status();
    let body = String::from_utf8_lossy(&res.into_body().try_into_bytes().unwrap()).into_owned();
    Ok((status, body))
}
