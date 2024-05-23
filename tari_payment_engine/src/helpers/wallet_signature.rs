use serde::{Deserialize, Serialize};
use tari_crypto::{
    hash_domain,
    ristretto::{RistrettoSchnorrWithDomain, RistrettoSecretKey},
};
use thiserror::Error;

use crate::{db_types::SerializedTariAddress, helpers::memo_signature::hex_to_schnorr};

hash_domain!(WalletSignatureDomain, "WalletSignature");

pub type WalletSchnorr = RistrettoSchnorrWithDomain<WalletSignatureDomain>;

#[derive(Debug, Clone, Error)]
#[error("Invalid wallet signature: {0}")]
pub struct WalletSignatureError(String);

impl From<String> for WalletSignatureError {
    fn from(e: String) -> Self {
        Self(e)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSignature {
    pub address: SerializedTariAddress,
    pub nonce: i64,
    pub signature: WalletSchnorr,
}

impl WalletSignature {
    pub fn create(
        address: SerializedTariAddress,
        nonce: i64,
        secret_key: &RistrettoSecretKey,
    ) -> Result<Self, WalletSignatureError> {
        let mut rng = rand::thread_rng();
        let message = signature_message(&address, nonce);
        let signature =
            WalletSchnorr::sign(secret_key, &message, &mut rng).map_err(|e| WalletSignatureError(e.to_string()))?;
        Ok(Self { address, nonce, signature })
    }

    pub fn new(address: &str, nonce: i64, signature: &str) -> Result<Self, WalletSignatureError> {
        let address = address.parse::<SerializedTariAddress>().map_err(|e| WalletSignatureError(e.to_string()))?;
        let signature = hex_to_schnorr::<_, WalletSignatureError>(signature)?;
        Ok(Self { address, nonce, signature })
    }

    pub fn is_valid(&self) -> bool {
        let message = signature_message(&self.address, self.nonce);
        let pubkey = self.address.as_address().public_key();
        self.signature.verify(pubkey, message)
    }

    pub fn as_json(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize WalletSignature")
    }
}

pub fn signature_message(address: &SerializedTariAddress, nonce: i64) -> String {
    format!("{}:{}", address.as_address(), nonce)
}
