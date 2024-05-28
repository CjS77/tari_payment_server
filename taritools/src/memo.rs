use tari_crypto::{ristretto::RistrettoSecretKey, tari_utilities::hex::Hex};
use tari_payment_engine::helpers::MemoSignature;

use crate::{keys::KeyInfo, MemoSignatureParams};

pub fn print_memo_signature(params: MemoSignatureParams) {
    let secret = match RistrettoSecretKey::from_hex(params.secret.as_str()) {
        Ok(sk) => sk,
        Err(e) => {
            println!("Invalid secret key: {e}");
            return;
        },
    };
    let key_info = KeyInfo::from_secret_key(secret, params.network);
    match MemoSignature::create(key_info.address(), params.order_id, &key_info.sk) {
        Ok(signature) => {
            println!("----------------------------- Memo Signature -----------------------------");
            println!("Wallet address: {}", key_info.address_as_hex());
            println!("Public key    : {:x}", &key_info.pk);
            println!("emoji id      : {}", key_info.address_as_emoji_string());
            println!("Secret        : {}", &key_info.sk.reveal().to_string());
            println!("Network       : {}", params.network);
            println!("auth: {}", signature.as_json());
            println!("------------------------------------------------------------------------");
        },
        Err(e) => eprintln!("Invalid input. {e}"),
    }
}
