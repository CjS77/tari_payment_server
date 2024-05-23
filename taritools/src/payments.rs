use tari_crypto::{ristretto::RistrettoSecretKey, tari_utilities::hex::Hex};
use tari_payment_engine::{
    db_types::{MicroTari, NewPayment, SerializedTariAddress},
    helpers::WalletSignature,
};

use crate::{keys::KeyInfo, PaymentAuthParams};

pub fn create_wallet_signature(info: &KeyInfo, nonce: i64, payload: &NewPayment) -> Result<WalletSignature, String> {
    let address = SerializedTariAddress::from(info.address().clone());
    // Create a wallet signature
    let wallet_signature = WalletSignature::create(address, nonce, &info.sk, payload).map_err(|e| e.to_string())?;
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
    let secret = match RistrettoSecretKey::from_hex(&params.secret) {
        Ok(sk) => sk,
        Err(e) => {
            println!("Invalid secret key: {}", e);
            return;
        },
    };
    let network = params.network;
    let key_info = KeyInfo::from_secret_key(secret, network);
    match create_wallet_signature(&key_info, params.nonce, &payment) {
        Ok(wallet_signature) => {
            println!("----------------------------- Wallet Auth -----------------------------");
            println!("Wallet address: {}", key_info.address_as_hex());
            println!("Public key    : {:x}", &key_info.pk);
            println!("emoji id      : {}", key_info.address_as_emoji_string());
            println!("Secret        : {}", &key_info.sk.reveal().to_string());
            println!("Network       : {}", network);
            println!("Nonce: {}", params.nonce);
            println!("auth: {}", wallet_signature.as_json());
            println!("payment: {}", serde_json::to_string(&payment).unwrap());
            println!("------------------------------------------------------------------------");
        },
        Err(e) => {
            println!("Error: {}", e);
        },
    }
}

fn build_payment(params: &PaymentAuthParams) -> Result<NewPayment, String> {
    let sender = params.sender.parse::<SerializedTariAddress>().map_err(|e| format!("Invalid sender address: {e}"))?;
    let amount = MicroTari::from_tari(params.amount);
    let memo = params.memo.clone();
    let order_id = params.order_id.clone();
    let txid = params.txid.clone();
    let payment = NewPayment { sender, amount, memo, order_id, txid };
    Ok(payment)
}
