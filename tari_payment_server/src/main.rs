use std::{env, str::FromStr};

use dotenvy::dotenv;
use log::{error, info};
use tari_common_types::tari_address::TariAddress;
use tari_payment_server::{
    cli::handle_command_line_args,
    config::{AuthConfig, ServerConfig},
    server::run_server,
};

#[actix_web::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    if handle_command_line_args() {
        return;
    }
    let config = ServerConfig::from_env_or_default();
    if !preflight_check(&config) {
        eprintln!("🚀️ Preflight check failed. Exiting. Check the logs for details.");
        return;
    }

    info!("🚀️ Starting server on {}:{}", config.host, config.port);
    match run_server(config).await {
        Ok(_) => println!("Bye!"),
        Err(e) => eprintln!("{e}"),
    }
}

fn preflight_check(config: &ServerConfig) -> bool {
    if env::var("TPG_SKIP_PREFLIGHT").ok() == Some("Yes".to_string()) {
        info!("🚦️ Skipping preflight checks. I hope you know what you're doing!");
        return true;
    }
    let mut result = true;
    info!("🚦️ Running preflight checks...");
    info!("🚦️ Checking for required environment variables...");
    if AuthConfig::try_from_env().is_err() {
        error!("🚦️ Preflight check FAILED: You must set up the JWT signing keys before carrying on.");
        result = false;
    }
    result &= match env::var("TPG_PAYMENT_WALLET_ADDRESS").ok() {
        Some(addr) => {
            let valid = TariAddress::from_str(&addr).is_ok();
            if !valid {
                error!(
                    "🚦️ TPG_PAYMENT_WALLET_ADDRESS is not a valid Tari address. Please set it to the address that \
                     customers send funds to."
                );
            }
            valid
        },
        None => {
            error!(
                "🚦️ TPG_PAYMENT_WALLET_ADDRESS is not set. This needs to be configured to the address that customers \
                 send funds to. If you don't set it, funds will be donated to the developers."
            );
            false
        },
    };
    if config.database_url.is_empty() {
        error!("🚦️ TPG_DATABASE_URL is not set. Please set it to the URL for the TPG database.");
        return false;
    }
    if result {
        info!("🚦️ Preflight check PASSED.");
    } else {
        error!("🚦️ Preflight check FAILED: Please fix the issues above before starting the server.");
        info!(
            "🚦️ If you really know what you're doing and want to skip the preflight check, set `TPG_SKIP_PREFLIGHT` \
             to `Yes` in your environment variables"
        );
    }
    result
}
