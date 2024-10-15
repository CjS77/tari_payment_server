use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use log::{debug, info, warn};
use regex::Regex;
use serde::Serialize;
use tari_common::configuration::Network;
use tari_crypto::{
    ristretto::RistrettoSecretKey,
    tari_utilities::{hex, hex::Hex},
};
use tari_payment_engine::{
    db_types::{NewPayment, OrderId, SerializedTariAddress},
    helpers::WalletSignature,
};
use tari_payment_server::{config::OrderIdField, data_objects::TransactionConfirmation};
use tpg_common::MicroTari;

use crate::{keys::KeyInfo, shopify, PaymentAuthParams, TxConfirmParams};

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
    /// The payment id as supplied by the hot wallet notifier. Note that this value is _not_ validated, therefore the
    /// chain of custody must be trusted between the hot wallet and this call.
    #[arg(long = "payment_id")]
    pub payment_id: Option<String>,
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
        let sender = params.sender.parse::<SerializedTariAddress>().unwrap_or_else(|e| {
            panic!("Invalid Tari Address in parameters. {e} Has the Tari address format changed? {}", params.sender)
        });
        let amount = params.amount;
        let memo = params.memo;
        let order_id = params.payment_id.and_then(|s| extract_order_id_from_payment_id(&s));
        let txid = params.txid;
        NewPayment { sender, amount, memo, order_id, txid }
    }
}

fn extract_order_id_from_payment_id(payment_id: &str) -> Option<OrderId> {
    if payment_id == "None" {
        debug!("No Payment id was provided");
        return None;
    }
    let open_data_regex = Regex::new(r"^data\((.*)\)$").expect("Invalid hardcoded regex");
    let address_and_data_regex = Regex::new(r"^address_and_data\((.*),(.*)\)$").expect("Invalid hardcoded regex");
    let hex = open_data_regex.captures(payment_id).and_then(|c| c.get(1));
    let hex = hex
        .or_else(|| address_and_data_regex.captures(payment_id).and_then(|c| c.get(2)))
        .or_else(|| {
            info!("Payment id was present but did not contain an order id: {payment_id}");
            None
        })?
        .as_str();
    debug!("Payment id info was extracted hex: {hex}");
    if hex.len() % 2 != 0 {
        warn!("Payment id hex had an odd number of characters: {hex}");
        return None;
    }
    let bytes = match hex::from_hex(hex) {
        Ok(b) => b,
        Err(e) => {
            warn!("Could not parse payment id hex: {e}");
            return None;
        },
    };
    let order_id = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => {
            warn!("Could not parse payment id bytes as utf8: {e}");
            return None;
        },
    };
    let format = shopify::order_id_field_from_env();
    order_id_from_payment_id_str(&order_id, format)
}

fn order_id_from_payment_id_str(payment_id_str: &str, format: OrderIdField) -> Option<OrderId> {
    if is_forbidden_pattern(payment_id_str) {
        return None;
    }
    match format {
        OrderIdField::Id => {
            debug!("We're configured to use the order id as the payment id");
            let order_id_regex =
                Regex::new(r#"^(order[\s_]?id[:=]\s*)?"?([^".]*)"?"#).expect("Invalid hardcoded regex");
            let s = order_id_regex.captures(payment_id_str).and_then(|c| c.get(2)).map(|s| s.as_str().trim())?;

            if s.is_empty() {
                warn!("Payment id was decoded correctly, but orderId was empty");
                None
            } else {
                info!("Payment id was decoded correctly, orderId: {s}");
                Some(OrderId::new(s))
            }
        },
        OrderIdField::Name => {
            debug!("We're configured to use the order name as the payment id");
            let order_id_regex = Regex::new(r#"#?(\d+)"#).expect("Invalid hardcoded regex");
            let s =
                order_id_regex.captures(payment_id_str).and_then(|c| c.get(1)).map(|s| format!("#{}", s.as_str()))?;
            Some(OrderId::new(s))
        },
    }
}

fn is_forbidden_pattern(payment_id_str: &str) -> bool {
    // Empty strings are not allowed
    if payment_id_str.is_empty() {
        warn!("Payment id was empty");
        return true;
    }
    // Check for phone numbers
    let regex = Regex::new(r#"^[^\d]*\d{3}-\d{3}-\d{4}$"#).expect("Invalid hardcoded regex");
    if regex.is_match(payment_id_str) {
        warn!("Payment id looked like a phone number: {payment_id_str}");
        return true;
    }
    false
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
            println!("Wallet address: {}", key_info.address_as_base58());
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
            println!("Wallet address: {}", key_info.address_as_base58());
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn extract_order_id() {
        env_logger::try_init().ok();
        assert!(extract_order_id_from_payment_id("").is_none());
        assert!(extract_order_id_from_payment_id("None").is_none());
        assert!(extract_order_id_from_payment_id("u64(12345)").is_none());
        // The payment id comes in hex-encoded
        assert!(extract_order_id_from_payment_id("data(12345)").is_none());
        assert!(extract_order_id_from_payment_id("order_id: 12345").is_none());
        // abcde1234568 is not valid UTF-8
        assert!(extract_order_id_from_payment_id("abcde1234568").is_none());
        // accidentally valid utf8
        assert_eq!(extract_order_id_from_payment_id("data(48656c6c6f20576f726c64)").unwrap().as_str(), "Hello World");
    }

    #[test]
    pub fn order_id_regex() {
        env_logger::try_init().ok();
        let matches = |actual: Option<OrderId>, expected: &str| actual.unwrap().as_str() == expected;
        assert!(matches(order_id_from_payment_id_str("order_id: 12345", OrderIdField::Id), "12345"));
        assert!(matches(order_id_from_payment_id_str("order_id=\"12345\"", OrderIdField::Id), "12345"));
        assert!(matches(order_id_from_payment_id_str("order_id: \"12345\"\n", OrderIdField::Id), "12345"));
        assert!(matches(order_id_from_payment_id_str("order_id=\"12345", OrderIdField::Id), "12345"));
        assert!(matches(order_id_from_payment_id_str("order_id= \" 12345\"\n", OrderIdField::Id), "12345"));
        assert!(matches(order_id_from_payment_id_str("order_id=126dbsa", OrderIdField::Id), "126dbsa"));
        assert!(matches(order_id_from_payment_id_str("order12345", OrderIdField::Id), "order12345"));
        assert!(matches(order_id_from_payment_id_str("order#12345", OrderIdField::Id), "order#12345"));
        // if order_id is actually part of the order id, the format must be like one of these:
        assert!(matches(
            order_id_from_payment_id_str("order_id:\"order_id#12345\"", OrderIdField::Id),
            "order_id#12345"
        ));
        assert!(matches(order_id_from_payment_id_str("order_id=order_id#12345", OrderIdField::Id), "order_id#12345"));
        assert!(matches(order_id_from_payment_id_str("123456", OrderIdField::Id), "123456"));
        assert!(matches(order_id_from_payment_id_str("\"123456\"", OrderIdField::Id), "123456"));
        assert!(matches(order_id_from_payment_id_str("\" ab123456cd \"", OrderIdField::Id), "ab123456cd"));
        assert!(order_id_from_payment_id_str("order_id=", OrderIdField::Id).is_none());
        assert!(order_id_from_payment_id_str("order_id=\"\"", OrderIdField::Id).is_none());
        assert!(order_id_from_payment_id_str("order_id=\"\"", OrderIdField::Id).is_none());
    }

    #[test]
    pub fn shopify_order_name() {
        env_logger::try_init().ok();
        let matches = |actual: Option<OrderId>, expected: &str| actual.unwrap().as_str() == expected;
        assert!(matches(order_id_from_payment_id_str("12345", OrderIdField::Name), "#12345"));
        assert!(matches(order_id_from_payment_id_str("#12345", OrderIdField::Name), "#12345"));
        assert!(matches(order_id_from_payment_id_str("#00001", OrderIdField::Name), "#00001"));
        assert!(matches(order_id_from_payment_id_str("Order#12345", OrderIdField::Name), "#12345"));
        assert!(matches(order_id_from_payment_id_str("Order# 12345", OrderIdField::Name), "#12345"));
        assert!(matches(order_id_from_payment_id_str("order#12345", OrderIdField::Name), "#12345"));
        assert!(matches(order_id_from_payment_id_str("order 12345", OrderIdField::Name), "#12345"));
        assert!(matches(order_id_from_payment_id_str("Order ID is 1234", OrderIdField::Name), "#1234"));
        assert!(matches(order_id_from_payment_id_str("Track Order 9876. This is 500", OrderIdField::Name), "#9876"));
        assert!(matches(order_id_from_payment_id_str("order 123 456", OrderIdField::Name), "#123"));
        assert!(matches(order_id_from_payment_id_str("123 45 67", OrderIdField::Name), "#123"));
        assert!(matches(order_id_from_payment_id_str("#5674 Main st", OrderIdField::Name), "#5674"));
        assert!(matches(order_id_from_payment_id_str("5674 Main st", OrderIdField::Name), "#5674"));
        assert!(matches(order_id_from_payment_id_str("#5674. Phone 555-666-1111", OrderIdField::Name), "#5674"));
        assert!(order_id_from_payment_id_str("555-888-9995", OrderIdField::Name).is_none());
        assert!(order_id_from_payment_id_str("phone: 555-888-9995", OrderIdField::Name).is_none());
        assert!(order_id_from_payment_id_str("Order#", OrderIdField::Name).is_none());
    }
}
