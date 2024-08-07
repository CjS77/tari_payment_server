use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use serde::Serialize;
use tari_common::configuration::Network;
use tari_crypto::{ristretto::RistrettoSecretKey, tari_utilities::hex::Hex};
use tari_payment_engine::{
    db_types::{NewPayment, SerializedTariAddress},
    helpers::WalletSignature,
};
use tari_payment_server::data_objects::TransactionConfirmation;
use tpg_common::MicroTari;

use crate::{keys::KeyInfo, PaymentAuthParams, TxConfirmParams};

#[derive(Debug, Subcommand)]
pub enum WalletCommand {
    Received(ReceivedPaymentParams),
    Confirmed(ConfirmationParams),
}

#[derive(Debug, Args)]
pub struct ReceivedPaymentParams {
    #[arg(short, long)]
    pub profile: String,
    #[arg(short, long, value_parser = parse_amount)]
    pub amount: MicroTari,
    #[arg(short, long)]
    pub txid: String,
    #[arg(short, long)]
    pub memo: Option<String>,
    #[arg(short, long)]
    pub sender: String,
}

fn parse_amount(s: &str) -> std::result::Result<MicroTari, String> {
    #[allow(clippy::cast_possible_truncation)]
    let value = s.parse::<i64>().or_else(|e| {
        if s.ends_with(" T") {
            s.trim_end_matches(" T").parse::<f64>().map(|v| (v * 1.0e6) as i64).map_err(|_| e.to_string())
        } else if s.ends_with(" uT") {
            s.trim_end_matches(" uT").parse::<i64>().map_err(|_| e.to_string())
        } else if s.ends_with(" µT") {
            s.trim_end_matches(" µT").parse::<i64>().map_err(|_| e.to_string())
        } else {
            Err(e.to_string())
        }
    })?;
    Ok(MicroTari::from(value))
}

impl From<ReceivedPaymentParams> for NewPayment {
    fn from(params: ReceivedPaymentParams) -> Self {
        let sender = params
            .sender
            .parse::<SerializedTariAddress>()
            .expect("Invalid Tari Address in parameters. Has the Tari address format changed?");
        let amount = params.amount;
        let memo = params.memo;
        let order_id = None;
        let txid = params.txid;
        NewPayment { sender, amount, memo, order_id, txid }
    }
}
#[derive(Debug, Args)]
pub struct ConfirmationParams {
    #[arg(short, long)]
    pub profile: String,
    #[arg(short, long)]
    pub txid: String,
}

pub fn create_wallet_signature<T: Serialize>(info: &KeyInfo, nonce: i64, payload: &T) -> Result<WalletSignature> {
    let address = SerializedTariAddress::from(info.address().clone());
    // Create a wallet signature
    let wallet_signature = WalletSignature::create(address, nonce, &info.sk, payload)?;
    Ok(wallet_signature)
}

pub fn print_payment_auth(params: PaymentAuthParams) {
    let payment = match build_payment(&params) {
        Ok(p) => p,
        Err(e) => {
            println!("Error: {e}");
            return;
        },
    };
    match build_auth(&params.secret, params.network, params.nonce, &payment) {
        Ok((wallet_signature, key_info)) => {
            println!("----------------------------- Wallet Auth -----------------------------");
            println!("Wallet address: {}", key_info.address_as_hex());
            println!("Public key    : {:x}", &key_info.pk);
            println!("emoji id      : {}", key_info.address_as_emoji_string());
            println!("Secret        : {}", &key_info.sk.reveal().to_string());
            println!("Network       : {}", params.network);
            println!("Nonce: {}", params.nonce);
            println!("auth: {}", wallet_signature.as_json());
            println!(
                "payment: {}",
                serde_json::to_string(&payment).unwrap_or_else(|e| format!("Could not represent payment as JSON. {e}"))
            );
            println!("------------------------------------------------------------------------");
        },
        Err(e) => {
            println!("Error: {}", e);
        },
    }
}

fn build_payment(params: &PaymentAuthParams) -> Result<NewPayment> {
    let sender = params.sender.parse::<SerializedTariAddress>()?;
    let amount = MicroTari::from_tari(params.amount);
    let memo = params.memo.clone();
    let order_id = params.order_id.clone();
    let txid = params.txid.clone();
    let payment = NewPayment { sender, amount, memo, order_id, txid };
    Ok(payment)
}

fn build_auth<T: Serialize>(
    secret: &str,
    network: Network,
    nonce: i64,
    payment: &T,
) -> Result<(WalletSignature, KeyInfo)> {
    let secret = match RistrettoSecretKey::from_hex(secret) {
        Ok(sk) => sk,
        Err(e) => {
            return Err(anyhow!("Invalid secret key: {e}"));
        },
    };
    let key_info = KeyInfo::from_secret_key(secret, network);
    let sig = create_wallet_signature(&key_info, nonce, &payment)?;
    Ok((sig, key_info))
}

pub fn print_tx_confirm(params: TxConfirmParams) {
    let confirmation = TransactionConfirmation { txid: params.txid.clone() };
    match build_auth(&params.secret, params.network, params.nonce, &confirmation) {
        Ok((wallet_signature, key_info)) => {
            println!("----------------------------- Wallet Auth -----------------------------");
            println!("Wallet address: {}", key_info.address_as_hex());
            println!("Public key    : {:x}", &key_info.pk);
            println!("emoji id      : {}", key_info.address_as_emoji_string());
            println!("Secret        : {}", &key_info.sk.reveal().to_string());
            println!("Network       : {}", params.network);
            println!("Nonce: {}", params.nonce);
            println!("auth: {}", wallet_signature.as_json());
            println!(
                "confirmation: {}",
                serde_json::to_string(&confirmation)
                    .unwrap_or_else(|e| format!("Could not represent confirmation as JSON. {e}"))
            );
            println!("------------------------------------------------------------------------");
        },
        Err(e) => {
            println!("Error: {}", e);
        },
    }
}
