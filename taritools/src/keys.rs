use rand::thread_rng;
use tari_common::configuration::Network;
use tari_common_types::tari_address::{TariAddress, TariAddressFeatures};
use tari_crypto::{
    keys::PublicKey,
    ristretto::{RistrettoPublicKey, RistrettoSecretKey},
};

pub struct KeyInfo {
    pub sk: RistrettoSecretKey,
    pub pk: RistrettoPublicKey,
    pub network: Network,
}

impl KeyInfo {
    pub fn random(network: Network) -> Self {
        let (sk, pk) = RistrettoPublicKey::random_keypair(&mut thread_rng());
        Self { sk, pk, network }
    }

    pub fn from_secret_key(secret_key: RistrettoSecretKey, network: Network) -> Self {
        let pk = RistrettoPublicKey::from_secret_key(&secret_key);
        Self { sk: secret_key, pk, network }
    }

    pub fn address(&self) -> TariAddress {
        TariAddress::new_single_address(self.pk.clone(), self.network, TariAddressFeatures::default())
    }

    #[deprecated(since = "0.3.0", note = "Use as_base58 instead")]
    pub fn address_as_hex(&self) -> String {
        self.address().to_hex()
    }

    pub fn address_as_base58(&self) -> String {
        self.address().to_base58()
    }

    pub fn address_as_emoji_string(&self) -> String {
        self.address().to_emoji_string()
    }
}
