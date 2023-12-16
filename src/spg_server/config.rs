use log::*;
use std::env;

const DEFAULT_SPG_HOST: &str = "127.0.0.1";
const DEFAULT_SPG_PORT: u16 = 8360;

pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub shopify_api_key: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_SPG_HOST.to_string(),
            port: DEFAULT_SPG_PORT,
            shopify_api_key: String::default(),
        }
    }
}

impl ServerConfig {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            ..Default::default()
        }
    }

    pub fn from_env_or_default() -> Self {
        let host = env::var("SPG_HOST")
            .ok()
            .unwrap_or_else(|| DEFAULT_SPG_HOST.into());
        let port = env::var("SPG_PORT")
            .map(|s| {
                s.parse::<u16>().unwrap_or_else(|e| {
                    error!(
                        "{s} is not a valid port for SPG_PORT. {e} Using the default, {DEFAULT_SPG_PORT}, \
                         instead."
                    );
                    DEFAULT_SPG_PORT
                })
            })
            .ok()
            .unwrap_or(DEFAULT_SPG_PORT);
        let shopify_api_key = env::var("SPG_SHOPIFY_API_KEY")
            .ok()
            .unwrap_or_else(|| {
                error!("SPG_SHOPIFY_API_KEY is not set. Please set it to the API key for your Shopify app.");
                String::default()
            });
        Self {
            host,
            port,
            shopify_api_key,
        }
    }
}
