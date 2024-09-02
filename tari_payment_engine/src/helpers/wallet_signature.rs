use serde::{Deserialize, Serialize};
use tari_crypto::{
    hash_domain,
    ristretto::{RistrettoSchnorrWithDomain, RistrettoSecretKey},
};
use thiserror::Error;

use crate::{
    db_types::SerializedTariAddress,
    helpers::memo_signature::{de_sig, hex_to_schnorr, ser_sig},
};

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
    #[serde(serialize_with = "ser_sig", deserialize_with = "de_sig")]
    pub signature: WalletSchnorr,
}

impl WalletSignature {
    pub fn create<T: Serialize>(
        address: SerializedTariAddress,
        nonce: i64,
        secret_key: &RistrettoSecretKey,
        payload: &T,
    ) -> Result<Self, WalletSignatureError> {
        let mut rng = rand::thread_rng();
        let message = Self::signature_message(&address, nonce, &payload)?;
        let signature =
            WalletSchnorr::sign(secret_key, &message, &mut rng).map_err(|e| WalletSignatureError(e.to_string()))?;
        Ok(Self { address, nonce, signature })
    }

    pub fn signature_message<T: Serialize>(
        address: &SerializedTariAddress,
        nonce: i64,
        payload: &T,
    ) -> Result<String, WalletSignatureError> {
        let msg_payload = serde_json::to_string(payload)
            .map_err(|e| WalletSignatureError(format!("Could not serialize wallet signature payload. {e}")))?;
        Ok(format!("{}:{}:{}", address.as_address(), nonce, msg_payload))
    }

    pub fn new(address: &str, nonce: i64, signature: &str) -> Result<Self, WalletSignatureError> {
        let address = address.parse::<SerializedTariAddress>().map_err(|e| WalletSignatureError(e.to_string()))?;
        let signature = hex_to_schnorr::<_, WalletSignatureError>(signature)?;
        Ok(Self { address, nonce, signature })
    }

    /// Verify the signature against the address and nonce. This does *not* verify that the wallet is the wallet we
    /// think it is, only that the signature is valid for the given address and nonce.
    ///
    /// You will still need to fetch the nonce and IP address from the database for the given address
    /// and check that they match the expected values.
    pub fn is_valid<T: Serialize>(&self, payload: &T) -> bool {
        let Ok(message) = Self::signature_message(&self.address, self.nonce, payload) else {
            return false;
        };
        let pubkey = self.address.as_address().comms_public_key();
        self.signature.verify(pubkey, message)
    }

    pub fn as_json(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize WalletSignature")
    }
}
