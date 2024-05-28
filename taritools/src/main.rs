use clap::{Args, Parser, Subcommand};
use tari_common::configuration::Network;

mod jwt_token;
mod keys;

mod memo;
mod payments;

use jwt_token::print_jwt_token;
use tari_payment_engine::db_types::OrderId;

use crate::{
    memo::print_memo_signature,
    payments::{print_payment_auth, print_tx_confirm},
};

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

fn main() {
    let cli = Arguments::parse();
    match cli.command {
        Command::NewAddress => print_new_address(cli.network),
        Command::AccessToken { secret, network, roles } => print_jwt_token(secret, network, roles),
        Command::MemoSignature(params) => print_memo_signature(params),
        Command::PaymentAuth(params) => print_payment_auth(params),
        Command::TxConfirm(params) => print_tx_confirm(params),
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
