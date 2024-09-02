use clap::{error::ErrorKind, Args, Parser, Subcommand};
use dotenvy::dotenv;
use tari_common::configuration::Network;

mod interactive;
mod jwt_token;
mod keys;
mod profile_manager;

mod memo;
mod payments;
mod setup;
mod shopify;

mod tari_payment_server;
mod wallet;

use jwt_token::print_jwt_token;
use log::*;
use tari_payment_engine::db_types::OrderId;

use crate::{
    interactive::InteractiveApp,
    memo::print_memo_signature,
    payments::{print_payment_auth, print_tx_confirm, WalletCommand},
    setup::{handle_setup_command, SetupCommand},
    shopify::{handle_shopify_command, ShopifyCommand},
    wallet::handle_wallet_command,
};

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
#[derive(Parser, Debug)]
#[command(version = "1.0.0", author = "CjS77")]
pub struct Arguments {
    /// The network to use (nextnet, stagenet, mainnet). Default is mainnet.
    #[arg(short, long, default_value = "mainnet")]
    network: Network,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate and print a new random secret key and print the associated public key, Tari address, and emoji id.
    #[clap(name = "address")]
    NewAddress,
    /// Generate a JWT token for use in authenticating with the Tari Payment Server (e.g. for Curl or Postman).
    /// Usually, it's much easier to use the interactive mode of Taritools and let it handle authentication for
    /// you.
    ///
    /// When using this command, you can specify the roles desired for the login token (assuming they've been granted
    /// on the server of course).
    #[clap(name = "token")]
    AccessToken {
        /// The secret key to use for the token
        #[arg(short = 's', long = "seckey")]
        secret: String,
        /// The network to use (nextnet, stagenet, mainnet). Default is mainnet.
        #[arg(short = 'n', long = "network", default_value = "mainnet")]
        network: Network,
        /// Roles you want the token to grant
        #[arg(short = 'r', long = "roles", default_value = "user")]
        roles: Vec<String>,
    },
    /// Generate a memo signature in order to claim orders in storefronts.
    ///
    /// This is useful to paste into the memo field of a payment from the console wallet, but it's generally far more
    /// convenient to use the 'Claim order' command in the interactive mode of Taritools.
    #[clap(name = "memo")]
    MemoSignature(MemoSignatureParams),
    /// Generate a payment authorization signature to acknowledge a payment to a hot wallet.
    ///
    /// This command will very seldom be used directly outside of testing.
    #[clap(name = "payment")]
    PaymentAuth(PaymentAuthParams),
    /// Generate a transaction confirmation signature to confirm a payment to a hot wallet.
    ///
    /// This command will very seldom be used directly outside of testing.
    #[clap(name = "confirm")]
    TxConfirm(TxConfirmParams),
    /// Commands for interacting with Shopify storefronts.
    ///
    /// There are several subcommands for interacting with your shopify store. Your environment (or `.env` file)
    /// must contain the following variables:
    ///
    /// * `TPG_SHOPIFY_SHOP`: Your Shopify shop name, e.g. `my-shop.myshopify.com`
    /// * `TPG_SHOPIFY_API_VERSION`: Optional. The API version to use. Default is `2024-04`.
    /// * `TPG_SHOPIFY_STOREFRONT_ACCESS_TOKEN`: Your Shopify storefront access token. e.g. `yyyyyyyyy`
    /// * `TPG_SHOPIFY_ADMIN_ACCESS_TOKEN`: Your Shopify admin access token. e.g. `shpat_xxxxxxxx`
    /// * `TPG_SHOPIFY_API_SECRET`: Your Shopify API secret. e.g. `aaaaaaaaaaaa`
    ///
    /// Not all of these environment variables are required for all commands, but the `TPG_SHOPIFY_ADMIN_ACCESS_TOKEN`
    ///  and `TPG_SHOPIFY_SHOP` are required for most of the important administrative commands in taritools.
    #[command(subcommand, verbatim_doc_comment)]
    Shopify(ShopifyCommand),
    /// Commands created for use by console wallet to communicate with the Tari Payment Server.
    ///
    /// See `tps_notify.sh`.
    #[command(subcommand)]
    Wallet(WalletCommand),
    #[command(subcommand)]
    /// Commands for helping in setting up the Tari Payment Server.
    Setup(SetupCommand),
}

#[derive(Debug, Args)]
pub struct PaymentAuthParams {
    /// The payment wallet's secret key
    #[arg(short = 's', long = "seckey")]
    secret: String,
    /// The network to use (testnet, stagenet, mainnet)
    #[arg(short = 'n', long = "network", default_value = "mainnet")]
    network: Network,
    /// A monotonically increasing nonce.
    ///
    /// The current Unix epoch is a good stateless means of generating a nonce, assuming the calls aren't made more
    /// than once per second.
    #[arg(short = 'c', long = "nonce", default_value = "1")]
    nonce: i64,
    /// The amount of the payment, in Tari
    #[arg(short = 'a', long = "amount", default_value = "250")]
    amount: i64,
    /// The transaction identifier. Typically, the kernel signature in Tari
    #[arg(short = 't', long = "txid", default_value = "payment001")]
    txid: String,
    /// The memo attached to the transfer
    #[arg(short = 'm', long = "memo")]
    memo: Option<String>,
    /// The order number associated with this payment. Generally extracted from the memo.
    #[arg(short = 'o', long = "order")]
    order_id: Option<OrderId>,
    /// The sender's address
    #[arg(short = 'x', long = "sender")]
    sender: String,
}

#[derive(Debug, Args)]
pub struct MemoSignatureParams {
    /// The user's wallet secret key
    #[arg(short = 's', long = "seckey")]
    secret: String,
    /// The network to use (testnet, stagenet, mainnet)
    #[arg(short = 'n', long = "network", default_value = "mainnet")]
    network: Network,
    /// The order number associated with this payment. Generally extracted from the memo.
    #[arg(short = 'o', long = "order")]
    order_id: String,
}

#[derive(Debug, Args)]
pub struct TxConfirmParams {
    /// The payment wallet's secret key
    #[arg(short = 's', long = "seckey")]
    secret: String,
    /// The network to use (testnet, stagenet, mainnet)
    #[arg(short = 'n', long = "network", default_value = "mainnet")]
    network: Network,
    /// A monotonically increasing nonce
    #[arg(short = 'c', long = "nonce", default_value = "1")]
    nonce: i64,
    /// The transaction identifier. Typically, the kernel signature in Tari
    #[arg(short = 't', long = "txid", default_value = "payment001")]
    txid: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv().ok();
    match Arguments::try_parse() {
        Ok(cli) => run_command(cli).await,
        Err(e) => match e.kind() {
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => run_interactive().await,
            _ => println!("{e}"),
        },
    }
}

async fn run_command(cli: Arguments) {
    match cli.command {
        Command::NewAddress => print_new_address(cli.network),
        Command::AccessToken { secret, network, roles } => print_jwt_token(secret, network, roles),
        Command::MemoSignature(params) => print_memo_signature(params),
        Command::PaymentAuth(params) => print_payment_auth(params),
        Command::TxConfirm(params) => print_tx_confirm(params),
        Command::Shopify(shopify_command) => handle_shopify_command(shopify_command).await,
        Command::Wallet(wallet_command) => handle_wallet_command(wallet_command).await,
        Command::Setup(setup_command) => handle_setup_command(setup_command).await,
    }
}

async fn run_interactive() {
    println!(
        "No command given. If this was unintended, enter `CTRL-C` to exit and run `{APP_NAME} --help` to see a full \
         list of commands."
    );
    let mut app = InteractiveApp::new();
    match app.run().await {
        Ok(_) => println!("Bye!"),
        Err(e) => error!("Session ended with error: {}", e),
    }
}

fn print_new_address(network: Network) {
    let info = keys::KeyInfo::random(network);
    println!("----------------------------- Tari Address -----------------------------");
    println!("Network: {}", info.network);
    println!("Secret key: {}", info.sk.reveal());
    println!("Public key: {:x}", info.pk);
    println!("Address (hex): {}", info.address_as_hex());
    println!("Address      : {}", info.address_as_base58());
    println!("Emoji ID: {}", info.address_as_emoji_string());
    println!("------------------------------------------------------------------------");
}
