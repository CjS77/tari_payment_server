use blake2::{Blake2b512, Digest};
use tari_common::configuration::Network;
use tari_common_types::tari_address::TariAddress;
use tari_crypto::{ristretto::RistrettoPublicKey, tari_utilities::ByteArray};

/// Creates a dummy TariAddress for a given customer id. The address is created by hashing the customer id and
/// then setting the first 8 bytes to a specific prefix and the last byte to 0. The resulting hash is then
/// converted to a RistrettoPublicKey and used to create a TariAddress. Sometimes, the custom hash
/// may not be a valid RistrettoPublicKey, in which case the last 8 bytes are incremented by 1 and the process
/// is repeated until a valid RistrettoPublicKey is found.
///
/// The end result is that the dummy addresses are valid, and easily recognizable as dummy addresses,
/// since they all start with the same prefix, `000000ba5e4d0000`, and are deterministic for a given customer id.
///
/// If written as emoji ids, the prefix is 🌀🌀🌀💤🎽🎨🌀🌀
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
    TariAddress::new(key, Network::MainNet)
}

#[cfg(test)]
mod test {
    use rand::{distributions::Alphanumeric, Rng};

    use super::*;

    #[test]
    fn test_create_dummy_address_for_cust_id() {
        let address = create_dummy_address_for_cust_id("1234");
        assert_eq!(address.to_hex(), "000000ba5e4d0000b31de27536b81df7f005027d4f847667df13a0569b60480310");
        assert_eq!(address.to_emoji_string(), "🌀🌀🌀💤🎽🎨🌀🌀💎🍌😇🐗🍵💡🍌🚦🚑🌍🌈🐢🎪🐮🐘🏥🔱🍀👞🎳👗🎿🎢🌊🌹");
        let address = create_dummy_address_for_cust_id("5500221");
        assert_eq!(address.to_hex(), "000000ba5e4d0000879b5e3aa0cba1ffae4b48daf80b944962287abf35c0cc0398");
        let address = create_dummy_address_for_cust_id("orderid-X-67483:3321a/2024-05-01:18:08.004");
        assert_eq!(address.to_hex(), "000000ba5e4d00008eb731b31738fe74d7c5475687ca0dce26e03fbad621b803cb");
    }

    #[test]
    fn mini_fuzz() {
        for _ in 0..1000 {
            let id: String = rand::thread_rng().sample_iter(&Alphanumeric).take(16).map(char::from).collect();
            let address = create_dummy_address_for_cust_id(&id);
            assert!(address.to_hex().starts_with("000000ba5e4d0000"));
        }
    }
}
