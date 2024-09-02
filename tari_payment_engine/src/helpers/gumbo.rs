use std::str::FromStr;

use blake2::{Blake2b512, Digest};
use log::error;
use tari_common::configuration::Network;
use tari_common_types::tari_address::{TariAddress, TariAddressFeatures};
use tari_crypto::{ristretto::RistrettoPublicKey, tari_utilities::ByteArray};

pub const DONATION_WALLET_ADDRESS: &str = "143UtnSymZykCAm95xAKKKe8nowL6zif8qb1h7yHxgtD9XZ";

/// Creates a dummy TariAddress for a given customer id. The address is created by hashing the customer id and
/// then setting the first 8 bytes to a specific prefix and the last byte to 0. The resulting hash is then
/// converted to a RistrettoPublicKey and used to create a TariAddress. Sometimes, the custom hash
/// may not be a valid RistrettoPublicKey, in which case the last 8 bytes are incremented by 1 and the process
/// is repeated until a valid RistrettoPublicKey is found.
///
/// The end result is that the dummy addresses are valid, and easily recognizable as dummy addresses,
/// since the hex representations all start with the same prefix, `0002000000ba5e4d0000`, and are deterministic for a
/// given customer id.
///
/// If written as emoji ids, the prefix is 🐢🌈🐢🐢🐢💤🎽
/// As base58, the prefix is 13111eLuVvx
pub fn create_dummy_address_for_cust_id(cust_id: &str) -> TariAddress {
    let prefix = [0, 0, 0, 0xba, 0x5e, 0x4d, 0, 0];
    let mut cust_id_hash = Blake2b512::digest(cust_id.as_bytes()).to_vec();
    cust_id_hash[..8].copy_from_slice(&prefix);
    cust_id_hash[31] = 0;
    let mut key = RistrettoPublicKey::from_canonical_bytes(&cust_id_hash[..32]);
    while key.is_err() {
        let val = u64::from_be_bytes(cust_id_hash[24..32].try_into().unwrap()).wrapping_add(1);
        cust_id_hash[24..32].copy_from_slice(&val.to_be_bytes());
        key = RistrettoPublicKey::from_canonical_bytes(&cust_id_hash[..32]);
    }
    let key = key.unwrap();
    TariAddress::new_single_address(key, Network::MainNet, TariAddressFeatures::create_interactive_only())
}

/// Returns the Tari wallet address that should be used to make payments.
///
/// This value should be set in the environment variable `TPG_PAYMENT_WALLET_ADDRESS`.
/// If this is _not_ set, it will default to the developers' wallet, and we will gladly accept these payments :)
pub fn get_payment_wallet_address() -> TariAddress {
    std::env::var("TPG_PAYMENT_WALLET_ADDRESS")
        .ok()
        .and_then(|s| {
            TariAddress::from_str(&s)
                .map_err(|e| {
                    error!(
                        "Invalid TPG_PAYMENT_WALLET_ADDRESS: {e}. You should fix this immediately, because funds will \
                         be sent to the developers instead."
                    );
                })
                .ok()
        })
        .unwrap_or_else(|| TariAddress::from_str(DONATION_WALLET_ADDRESS).unwrap())
}

#[cfg(test)]
mod test {
    use rand::{distributions::Alphanumeric, Rng};

    use super::*;

    #[test]
    fn test_create_dummy_address_for_cust_id() {
        let address = create_dummy_address_for_cust_id("1234");
        assert_eq!(address.to_hex(), "0002000000ba5e4d0000b31de27536b81df7f005027d4f847667df13a0569b604803ad");
        assert_eq!(address.to_base58(), "13111eLuVvxBvAhWdTUMEKJRgpbFusby2KqBaGT5emkFzt");
        assert_eq!(address.to_emoji_string(), "🐢🌈🐢🐢🐢💤🎽🎨🐢🐢💎🍌😇🐗🍵💡🍌🚦🚑🐋🌈🦋🎪🐮🐘🏥🔱🍀👞🎳👗🎿🎢🌊💈");
        let address = create_dummy_address_for_cust_id("5500221");
        assert_eq!(address.to_hex(), "0002000000ba5e4d0000879b5e3aa0cba1ffae4b48daf80b944962287abf35c0cc0325");
        assert_eq!(address.to_base58(), "13111eLuVvxBcfGU3RoW9hoSeSzhsszc7mDG6wL3avdXQg");
        let address = create_dummy_address_for_cust_id("orderid-X-67483:3321a/2024-05-01:18:08.004");
        assert_eq!(address.to_hex(), "0002000000ba5e4d00008eb731b31738fe74d7c5475687ca0dce26e03fbad621b80376");
        assert_eq!(address.to_base58(), "13111eLuVvxBfXApWbkBesNSD4zzbV5WCXXnbD87bF6aT7");
    }

    #[test]
    fn mini_fuzz() {
        for _ in 0..1000 {
            let id: String = rand::thread_rng().sample_iter(&Alphanumeric).take(16).map(char::from).collect();
            let address = create_dummy_address_for_cust_id(&id);
            assert!(address.to_hex().starts_with("0002000000ba5e4d0000"));
        }
    }
}
