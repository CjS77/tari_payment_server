use std::{env, io::Write, net::SocketAddr};

use log::*;
use rand::thread_rng;
use serde_json::json;
use tari_jwt::{
    tari_crypto::{
        keys::PublicKey,
        ristretto::{RistrettoPublicKey, RistrettoSecretKey},
        tari_utilities::hex::Hex,
    },
    Ristretto256SigningKey,
    Ristretto256VerifyingKey,
};
use tempfile::NamedTempFile;

use crate::errors::ServerError;

const DEFAULT_TPG_HOST: &str = "127.0.0.1";
const DEFAULT_TPG_PORT: u16 = 8360;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub shopify_api_key: String,
    pub database_url: String,
    pub auth: AuthConfig,
    /// If supplied, requests against /shopify endpoints will be checked against a whitelist of Shopify IP addresses.
    pub shopify_whitelist: Option<Vec<SocketAddr>>,
    /// If true, the X-Forwarded-For header will be used to determine the client's IP address, rather than the
    /// connection's remote address.
    pub use_x_forwarded_for: bool,
    /// If true, the X-Forwarded-Proto header will be used to determine the client's protocol, rather than the
    /// connection's remote address.
    pub use_forwarded: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_TPG_HOST.to_string(),
            port: DEFAULT_TPG_PORT,
            shopify_api_key: String::default(),
            database_url: String::default(),
            auth: AuthConfig::default(),
            shopify_whitelist: None,
            use_x_forwarded_for: false,
            use_forwarded: false,
        }
    }
}

impl ServerConfig {
    pub fn new(host: &str, port: u16) -> Self {
        Self { host: host.to_string(), port, ..Default::default() }
    }

    pub fn from_env_or_default() -> Self {
        let host = env::var("TPG_HOST").ok().unwrap_or_else(|| DEFAULT_TPG_HOST.into());
        let port = env::var("TPG_PORT")
            .map(|s| {
                s.parse::<u16>().unwrap_or_else(|e| {
                    error!("{s} is not a valid port for TPG_PORT. {e} Using the default, {DEFAULT_TPG_PORT}, instead.");
                    DEFAULT_TPG_PORT
                })
            })
            .ok()
            .unwrap_or(DEFAULT_TPG_PORT);
        let shopify_api_key = env::var("TPG_SHOPIFY_API_KEY").ok().unwrap_or_else(|| {
            error!("TPG_SHOPIFY_API_KEY is not set. Please set it to the API key for your Shopify app.");
            String::default()
        });
        let auth = AuthConfig::try_from_env().unwrap_or_default();
        let database_url = env::var("TPG_DATABASE_URL").ok().unwrap_or_else(|| {
            error!("TPG_DATABASE_URL is not set. Please set it to the URL for the TPG database.");
            String::default()
        });
        let shopify_whitelist = env::var("TPG_SHOPIFY_IP_WHITELIST")
            .map(|s| {
                s.split(',')
                    .filter_map(|s| {
                        s.parse()
                            .map_err(|e| {
                                warn!("Ignoring invalid IP address ({s}) in TPG_SHOPIFY_IP_WHITELIST: {e}");
                                None::<SocketAddr>
                            })
                            .ok()
                    })
                    .collect::<Vec<_>>()
            })
            .ok();
        let use_x_forwarded_for = env::var("TPG_USE_X_FORWARDED_FOR").map(|s| &s == "1" || &s == "true").is_ok();
        let use_forwarded = env::var("TPG_USE_FORWARDED").map(|s| &s == "1" || &s == "true").is_ok();
        Self { host, port, shopify_api_key, auth, database_url, shopify_whitelist, use_forwarded, use_x_forwarded_for }
    }
}

//-------------------------------------------------  AuthConfig  -------------------------------------------------------
#[derive(Clone, Debug)]
pub struct AuthConfig {
    /// This is the secret key used to sign JWTs. It must be in hex format and be a valid Tari secret key.
    pub jwt_signing_key: Ristretto256SigningKey,
    /// This is the public key used to verify JWTs. It must be in hex format and be a valid Tari public key.
    /// It must be the public key corresponding to the `jwt_signing_key`.
    pub jwt_verification_key: Ristretto256VerifyingKey,
}

impl Default for AuthConfig {
    fn default() -> Self {
        let mut tmpfile = NamedTempFile::new().ok().and_then(|f| f.keep().ok());
        warn!(
            "🚨️🚨️🚨️ The JWT signing key has not been set. I'm using a random value for this session.DO NOT operate on \
             production like this since you may lose access to data. 🚨️🚨️🚨️"
        );
        let mut rng = thread_rng();
        let (sk, pk) = RistrettoPublicKey::random_keypair(&mut rng);
        match &mut tmpfile {
            Some((f, p)) => {
                let key_data = json!({
                    "jwt_signing_key": sk.to_hex(),
                    "jwt_verification_key": pk.to_hex(),
                })
                .to_string();
                match writeln!(f, "{key_data}") {
                    Ok(()) => warn!(
                        "🚨️🚨️🚨️ The JWT signing key for this session was written to {}. If this is a production \
                         instance, you are doing it wrong! Set the TPG_JWT_SIGNING_KEY and TPG_JWT_VERIFICATION_KEY \
                         environment variables instead. 🚨️🚨️🚨️",
                        p.to_str().unwrap_or("???")
                    ),
                    Err(e) => warn!("Could not write the JWT signing key to the temporary file. {e}"),
                }
            },
            None => {
                warn!("Could not create a temporary file to store the JWT signing key. ");
            },
        }
        Self { jwt_signing_key: Ristretto256SigningKey(sk), jwt_verification_key: Ristretto256VerifyingKey(pk) }
    }
}

impl AuthConfig {
    pub fn try_from_env() -> Result<Self, ServerError> {
        let jwt_sk_hex = env::var("TPG_JWT_SIGNING_KEY").map_err(|e| ServerError::ConfigurationError(e.to_string()))?;
        let jwt_pk_hex =
            env::var("TPG_JWT_VERIFICATION_KEY").map_err(|e| ServerError::ConfigurationError(e.to_string()))?;
        // Why have users specify the public key if we can just derive it from the private key?
        // The reason is that it's easy to share and/or look up the public key if it is stored in the configuration.
        let sk = RistrettoSecretKey::from_hex(&jwt_sk_hex)
            .map_err(|e| ServerError::ConfigurationError(format!("Invalid signing key in TPG_JWT_SIGNING_KEY: {e}")))?;
        let expected = RistrettoPublicKey::from_secret_key(&sk);
        let vk = RistrettoPublicKey::from_hex(&jwt_pk_hex).map_err(|e| {
            ServerError::ConfigurationError(format!("Invalid verification key in TPG_JWT_VERIFICATION_KEY: {e}"))
        })?;
        if vk == expected {
            Ok(Self { jwt_signing_key: Ristretto256SigningKey(sk), jwt_verification_key: Ristretto256VerifyingKey(vk) })
        } else {
            Err(ServerError::ConfigurationError(
                "The verification key does not match the signing key. Check your configuration.".to_string(),
            ))
        }
    }
}
