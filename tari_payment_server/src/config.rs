use std::{env, io::Write, net::IpAddr};

use actix_jwt_auth_middleware::FromRequest;
use chrono::Duration;
use log::*;
use rand::thread_rng;
use serde_json::json;
use shopify_tools::ShopifyConfig as ShopifyApiConfig;
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
use tpg_common::Secret;

use crate::errors::ServerError;

const DEFAULT_TPG_HOST: &str = "127.0.0.1";
const DEFAULT_TPG_PORT: u16 = 8360;
const DEFAULT_UNCLAIMED_ORDER_TIMEOUT: Duration = Duration::hours(2);
const DEFAULT_UNPAID_ORDER_TIMEOUT: Duration = Duration::hours(48);

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub auth: AuthConfig,
    /// If true, the X-Forwarded-For header will be used to determine the client's IP address, rather than the
    /// connection's remote address.
    pub use_x_forwarded_for: bool,
    /// If true, the X-Forwarded-Proto header will be used to determine the client's protocol, rather than the
    /// connection's remote address.
    pub use_forwarded: bool,
    /// When true, _only_ the order_id field will be used to identify orders. When _false_, either the order_id or the
    /// alt_id can be used.
    pub strict_mode: bool,
    /// If true, the server will not validate payment API calls against a whitelist of wallet addresses. **DANGER**
    pub disable_wallet_whitelist: bool,
    /// If true, the server will not require signed messages in memo fields, but will accept naked order ids.
    /// **DANGER**
    pub disable_memo_signature_check: bool,
    /// The time before an unclaimed order is considered abandoned and marked as expired.
    pub unclaimed_order_timeout: Duration,
    /// The time before an unpaid order is considered expired and marked as such.
    pub unpaid_order_timeout: Duration,
    /// Shopify storefront configuration
    pub shopify_config: ShopifyConfig,
}

#[derive(Clone, Debug, Default)]
pub struct ShopifyConfig {
    /// The url for the shopify storefront to use. e.g. "my-shop.myshopify.com"
    pub shop: String,
    pub api_key: String,
    pub api_secret: Secret<String>,
    pub hmac_secret: Secret<String>,
    pub api_version: String,
    pub hmac_checks: bool,
    /// If supplied, requests against /shopify endpoints will be checked against a whitelist of Shopify IP addresses.
    /// To explicitly disable the whitelist, set this to "false", "none", or "0".
    pub whitelist: Option<Vec<IpAddr>>,
    pub admin_access_token: Secret<String>,
    pub storefront_access_token: Secret<String>,
    pub order_id_field: OrderIdField,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_TPG_HOST.to_string(),
            port: DEFAULT_TPG_PORT,
            database_url: String::default(),
            auth: AuthConfig::default(),
            use_x_forwarded_for: false,
            use_forwarded: false,
            strict_mode: true,
            disable_wallet_whitelist: false,
            disable_memo_signature_check: false,
            unclaimed_order_timeout: DEFAULT_UNCLAIMED_ORDER_TIMEOUT,
            unpaid_order_timeout: DEFAULT_UNPAID_ORDER_TIMEOUT,
            shopify_config: ShopifyConfig::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum OrderIdField {
    Name,
    Id,
}

impl Default for OrderIdField {
    fn default() -> Self {
        Self::Id
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
                    error!(
                        "🪛️ {s} is not a valid port for TPG_PORT. {e} Using the default, {DEFAULT_TPG_PORT}, instead."
                    );
                    DEFAULT_TPG_PORT
                })
            })
            .ok()
            .unwrap_or(DEFAULT_TPG_PORT);
        let database_url = env::var("TPG_DATABASE_URL").ok().unwrap_or_else(|| {
            error!("🪛️ TPG_DATABASE_URL is not set. Please set it to the URL for the TPG database.");
            String::default()
        });
        let auth = AuthConfig::try_from_env().unwrap_or_else(|e| {
            warn!(
                "🪛️ Could not load the authentication configuration from environment variables. {e}. Reverting to the \
                 default configuration."
            );
            AuthConfig::default()
        });
        let shopify_config = ShopifyConfig::from_env_or_defaults();
        let use_x_forwarded_for =
            env::var("TPG_USE_X_FORWARDED_FOR").map(|s| &s == "1" || &s == "true").unwrap_or(false);
        let use_forwarded = env::var("TPG_USE_FORWARDED").map(|s| &s == "1" || &s == "true").unwrap_or(false);
        let disable_wallet_whitelist =
            env::var("TPG_DISABLE_WALLET_WHITELIST").map(|s| &s == "1" || &s == "true").unwrap_or(false);
        let strict_mode = env::var("TPG_STRICT_MODE").map(|s| &s != "0" && &s != "false").unwrap_or(true);
        let disable_memo_signature_check =
            env::var("TPG_DISABLE_MEMO_SIGNATURE_CHECK").map(|s| &s == "1" || &s == "true").unwrap_or(false);
        let (unclaimed_order_timeout, unpaid_order_timeout) = configure_order_timeouts();
        Self {
            host,
            port,
            shopify_config,
            auth,
            database_url,
            use_forwarded,
            use_x_forwarded_for,
            strict_mode,
            disable_wallet_whitelist,
            disable_memo_signature_check,
            unclaimed_order_timeout,
            unpaid_order_timeout,
        }
    }
}

impl ShopifyConfig {
    pub fn from_env_or_defaults() -> Self {
        let api_config = ShopifyApiConfig::new_from_env_or_default();
        let api_key = env::var("TPG_SHOPIFY_API_KEY").ok().unwrap_or_else(|| {
            error!("🪛️ TPG_SHOPIFY_API_KEY is not set. Please set it to the API key for your Shopify app.");
            String::default()
        });
        let hmac_secret = env::var("TPG_SHOPIFY_HMAC_SECRET").ok().unwrap_or_else(|| {
            error!(
                "🪛️ TPG_SHOPIFY_HMAC_SECRET is not set. Please set it to the HMAC signing key for your Shopify app."
            );
            String::default()
        });
        let hmac_secret = Secret::new(hmac_secret);
        let hmac_checks = env::var("TPG_SHOPIFY_HMAC_CHECKS").map(|s| &s == "1" || &s == "true").unwrap_or(true);
        let whitelist = env::var("TPG_SHOPIFY_IP_WHITELIST").ok().and_then(|s| {
            if ["none", "false", "0"].contains(&s.to_lowercase().as_str()) {
                info!(
                    "🪛️ Shopify IP whitelist is disabled. If this is not what you want, set TPG_SHOPIFY_IP_WHITELIST \
                     to a comma-separated list of IP addresses to enable it."
                );
                return None;
            }
            let ip_addrs = s
                .split(',')
                .filter_map(|s| {
                    s.parse()
                        .map_err(|e| {
                            warn!("🪛️ Ignoring invalid IP address ({s}) in TPG_SHOPIFY_IP_WHITELIST: {e}");
                            None::<IpAddr>
                        })
                        .ok()
                })
                .collect::<Vec<IpAddr>>();
            Some(ip_addrs)
        });
        match &whitelist {
            Some(whitelist) if whitelist.is_empty() => {
                warn!(
                    "🚨️ The Shopify IP whitelist was configured, but is empty.  The server will run, but won't \
                     authorise any Shopify incoming requests."
                );
            },
            None => {
                info!("🪛️ No Shopify IP whitelist is set. Only HMAC validation will be used.");
            },
            Some(v) => {
                let addrs = v.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ");
                info!("🪛️ Shopify IP whitelist: {addrs}");
            },
        }
        let order_id_field = match env::var("TPG_SHOPIFY_ORDER_ID_FIELD").map(|s| s.to_lowercase()) {
            Ok(s) if s == "name" => OrderIdField::Name,
            Ok(s) if s == "id" => OrderIdField::Id,
            _ => {
                warn!("TPG_SHOPIFY_ORDER_ID_FIELD not set, using 'id' as default");
                OrderIdField::Id
            },
        };
        Self {
            shop: api_config.shop,
            api_version: api_config.api_version,
            api_key,
            api_secret: api_config.shared_secret,
            hmac_secret,
            hmac_checks,
            whitelist,
            admin_access_token: api_config.admin_access_token,
            storefront_access_token: api_config.storefront_access_token,
            order_id_field,
        }
    }

    pub fn shopify_api_config(&self) -> ShopifyApiConfig {
        ShopifyApiConfig {
            shop: self.shop.clone(),
            api_version: self.api_version.clone(),
            shared_secret: self.api_secret.clone(),
            admin_access_token: self.admin_access_token.clone(),
            storefront_access_token: self.storefront_access_token.clone(),
        }
    }
}

fn configure_order_timeouts() -> (Duration, Duration) {
    let unclaimed_order_timeout = env::var("TPG_UNCLAIMED_ORDER_TIMEOUT")
        .map_err(|_| {
            info!(
                "🪛️ TPG_UNCLAIMED_ORDER_TIMEOUT is not set. Using the default value of {} hrs.",
                DEFAULT_UNCLAIMED_ORDER_TIMEOUT.num_hours()
            )
        })
        .and_then(|s| {
            s.parse::<i64>()
                .map(Duration::hours)
                .map_err(|e| warn!("🪛️ Invalid configuration value for TPG_UNCLAIMED_ORDER_TIMEOUT. {e}"))
        })
        .ok()
        .unwrap_or(DEFAULT_UNCLAIMED_ORDER_TIMEOUT);
    let unpaid_order_timeout = env::var("TPG_UNPAID_ORDER_TIMEOUT")
        .map_err(|_| {
            info!(
                "🪛️ TPG_UNPAID_ORDER_TIMEOUT is not set. Using the default value of {} hrs.",
                DEFAULT_UNPAID_ORDER_TIMEOUT.num_hours()
            )
        })
        .and_then(|s| {
            s.parse::<i64>()
                .map(Duration::hours)
                .map_err(|e| warn!("🪛️ Invalid configuration value for TPG_UNPAID_ORDER_TIMEOUT. {e}"))
        })
        .ok()
        .unwrap_or(DEFAULT_UNPAID_ORDER_TIMEOUT);
    (unclaimed_order_timeout, unpaid_order_timeout)
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
                    Err(e) => warn!("🪛️ Could not write the JWT signing key to the temporary file. {e}"),
                }
            },
            None => {
                warn!("🪛️ Could not create a temporary file to store the JWT signing key. ");
            },
        }
        Self { jwt_signing_key: Ristretto256SigningKey(sk), jwt_verification_key: Ristretto256VerifyingKey(pk) }
    }
}

impl AuthConfig {
    pub fn try_from_env() -> Result<Self, ServerError> {
        let jwt_sk_hex = env::var("TPG_JWT_SIGNING_KEY")
            .map_err(|e| ServerError::ConfigurationError(format!("{e} [TPG_JWT_SIGNING_KEY]")))?;
        let jwt_pk_hex = env::var("TPG_JWT_VERIFICATION_KEY")
            .map_err(|e| ServerError::ConfigurationError(format!("{e} [TPG_JWT_VERIFICATION_KEY]")))?;
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

//-------------------------------------------------  ServerOptions  ----------------------------------------------------
/// A subset of the server configuration that is used to configure the server's behaviour. Generally we try to keep this
/// as small as possible, and exclude secrets to avoid passing sensitive information around the system.
#[derive(Clone, Copy, Debug, FromRequest)]
pub struct ServerOptions {
    pub use_x_forwarded_for: bool,
    pub use_forwarded: bool,
    pub disable_wallet_whitelist: bool,
    pub disable_memo_signature_check: bool,
    pub shopify_order_field: OrderIdField,
    pub strict_mode: bool,
}

impl ServerOptions {
    pub fn from_config(config: &ServerConfig) -> Self {
        Self {
            use_x_forwarded_for: config.use_x_forwarded_for,
            use_forwarded: config.use_forwarded,
            disable_wallet_whitelist: config.disable_wallet_whitelist,
            disable_memo_signature_check: config.disable_memo_signature_check,
            shopify_order_field: config.shopify_config.order_id_field,
            strict_mode: config.strict_mode,
        }
    }
}
