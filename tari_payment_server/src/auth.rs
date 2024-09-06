use std::time::Duration;

use actix_jwt_auth_middleware::{Authority, FromRequest, TokenSigner};
use actix_web::{error::Error as ActixWebError, Handler};
use log::*;
use serde::{Deserialize, Serialize};
use tari_common_types::tari_address::TariAddress;
use tari_jwt::{
    jwt_compact::{AlgorithmExt, Claims, Header, UntrustedToken},
    Ristretto256,
    Ristretto256SigningKey,
    Ristretto256VerifyingKey,
};
use tari_payment_engine::db_types::{LoginToken, Roles};

use crate::{config::AuthConfig, errors::AuthError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRequest)]
pub struct JwtClaims {
    pub address: TariAddress,
    pub roles: Roles,
}

pub type TpsAuthority = Authority<JwtClaims, Ristretto256, impl Handler<(), Output = Result<(), ActixWebError>>, ()>;

pub fn build_jwt_signer(jwt_signing_key: Ristretto256SigningKey) -> TokenSigner<JwtClaims, Ristretto256> {
    let header = Header::empty().with_token_type("JWT");
    TokenSigner::new()
        .signing_key(jwt_signing_key)
        .algorithm(Ristretto256)
        .header(header)
        .build()
        .expect("Failed to build token signer")
}
pub fn build_tps_authority(auth_config: AuthConfig) -> TpsAuthority {
    let AuthConfig { jwt_signing_key, jwt_verification_key } = auth_config;
    let token_signer = build_jwt_signer(jwt_signing_key);
    Authority::<JwtClaims, Ristretto256, _, _>::new()
        .refresh_authorizer(|| async { Ok(()) })
        .enable_header_tokens(true)
        .renew_access_token_automatically(false)
        .access_token_name("tpg_access_token")
        .refresh_token_name("tpg_refresh_token")
        .algorithm(Ristretto256)
        .verifying_key(jwt_verification_key)
        .token_signer(Some(token_signer))
        .build()
        .expect("Failed to build authority")
}

pub fn check_login_token_signature<S: AsRef<str>>(token: S) -> Result<LoginToken, AuthError> {
    let untrusted_token =
        UntrustedToken::new(token.as_ref()).map_err(|e| AuthError::PoorlyFormattedToken(format!("{e:?}")))?;
    let claims: Claims<LoginToken> =
        untrusted_token.deserialize_claims_unchecked().map_err(|e| AuthError::ValidationError(format!("{e:?}")))?;
    let pubkey = claims.custom.address.public_spend_key();
    let verifying_key = Ristretto256VerifyingKey(pubkey.clone());
    let (header, claims) = Ristretto256
        .validator(&verifying_key)
        .validate(&untrusted_token)
        .map_err(|e| AuthError::ValidationError(format!("{e}")))?
        .into_parts();
    trace!("Login token validated successfully. Header: {header:?}. Claims: {claims:?}");
    Ok(claims.custom)
}

pub struct TokenIssuer {
    signer: TokenSigner<JwtClaims, Ristretto256>,
}

impl TokenIssuer {
    pub fn new(config: &AuthConfig) -> Self {
        let signer = build_jwt_signer(config.jwt_signing_key.clone());
        Self { signer }
    }

    /// Issue a new access token for the given login token
    /// This method DOES NOT verify that the `login_token` contains legitimate information.
    /// This must be done prior to calling `issue_token`.
    pub fn issue_token(&self, login_token: LoginToken, duration: Option<Duration>) -> Result<String, AuthError> {
        let claim = JwtClaims { address: login_token.address, roles: login_token.desired_roles };
        let duration = duration.unwrap_or_else(|| Duration::from_secs(60 * 60 * 24));
        let token = self
            .signer
            .create_signed_token(&claim, duration)
            .map_err(|e| AuthError::ValidationError(format!("{e:?}")))?;
        Ok(token)
    }
}
