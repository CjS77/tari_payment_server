use clap::{error::ErrorKind, Args, Parser, Subcommand};
use dotenvy::dotenv;
use tari_common::configuration::Network;

mod interactive;
mod jwt_token;
mod keys;
mod profile_manager;

mod memo;
mod payments;
mod shopify;

mod tari_payment_server;

use jwt_token::print_jwt_token;
use log::*;
use tari_payment_engine::db_types::OrderId;

use crate::{
    memo::print_memo_signature,
    payments::{print_payment_auth, print_tx_confirm},
    shopify::{handle_shopify_command, ShopifyCommand},
};

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
#[derive(Parser, Debug)]
#[command(version = "1.0.0", author = "CjS77")]
pub struct Arguments {
    #[arg(short, long, default_value = "mainnet")]
    network: Network,
    /// Generate a new random Tari address
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[clap(name = "address")]
    NewAddress,
    #[clap(name = "token")]
    AccessToken {
        /// The secret key to use for the token
        #[arg(short = 's', long = "seckey")]
        secret: String,
        #[arg(short = 'n', long = "network", default_value = "mainnet")]
        network: Network,
        /// Roles you want the token to grant
        #[arg(short = 'r', long = "roles", default_value = "user")]
        roles: Vec<String>,
    },
    #[clap(name = "memo", about = "Generate a memo signature in order to authenticate orders in storefronts")]
    MemoSignature(MemoSignatureParams),
    #[clap(name = "payment")]
    PaymentAuth(PaymentAuthParams),
    #[clap(name = "confirm")]
    TxConfirm(TxConfirmParams),
    #[command(subcommand)]
    Shopify(ShopifyCommand),
}

#[derive(Debug, Args)]
pub struct PaymentAuthParams {
    /// The payment wallet's secret key
    #[arg(short = 's', long = "seckey")]
    secret: String,
    #[arg(short = 'n', long = "network", default_value = "mainnet")]
    /// The network to use (testnet, stagenet, mainnet)
    network: Network,
    /// A monotonically increasing nonce
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
    #[arg(short = 'n', long = "network", default_value = "mainnet")]
    /// The network to use (testnet, stagenet, mainnet)
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
    #[arg(short = 'n', long = "network", default_value = "mainnet")]
    /// The network to use (testnet, stagenet, mainnet)
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
    }
}

async fn run_interactive() {
    println!(
        "No command given. If this was unintended, enter `CTRL-C` to exit and run `{APP_NAME} --help` to see a full \
         list of commands."
    );
    match interactive::run().await {
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
    println!("Address: {}", info.address_as_hex());
    println!("Emoji ID: {}", info.address_as_emoji_string());
    println!("------------------------------------------------------------------------");
}
