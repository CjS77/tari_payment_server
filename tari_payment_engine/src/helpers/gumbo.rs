use blake2::{Blake2b512, Digest};
use tari_common_types::tari_address::{TariAddress};
use tari_crypto::ristretto::RistrettoPublicKey;
use tari_crypto::tari_utilities::ByteArray;

pub fn create_dummy_address_for_cust_id(cust_id: &str) -> TariAddress {
    let prefix = "ba5e4dd000000000";
    let mut cust_id_hash = Blake2b512::digest(cust_id.as_bytes()).to_vec();
    cust_id_hash[..].copy_from_slice(&prefix.as_bytes());
    let key = RistrettoPublicKey::from_canonical_bytes(&cust_id_hash).unwrap();
    TariAddress::new(key, Network::MainNet)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_create_dummy_address_for_cust_id() {
        let address = create_dummy_address_for_cust_id("1234");
        assert_eq!(address.to_string(), "ba5e4dd000000000b3b0b4");
    }
}
