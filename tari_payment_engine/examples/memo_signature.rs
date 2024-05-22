use tari_common_types::tari_address::TariAddress;
use tari_crypto::{ristretto::RistrettoSecretKey, tari_utilities::hex::Hex};
use tari_payment_engine::helpers::MemoSignature;

fn main() {
    let mut args = std::env::args();
    args.next(); // executable name
    let Some(address) = args.next().and_then(|s| s.parse::<TariAddress>().ok()) else {
        println!("Address is required");
        return;
    };
    let Some(order_id) = args.next() else {
        println!("Order ID is required");
        return;
    };
    let Some(secret_key) = args.next().and_then(|k| RistrettoSecretKey::from_hex(&k).ok()) else {
        println!("Secret key is required");
        return;
    };

    match MemoSignature::create(address, order_id, &secret_key) {
        Ok(signature) => {
            println!("Memo signature: {}", signature.as_json());
        },
        Err(e) => eprintln!("Invalid input. {e}"),
    }
}
