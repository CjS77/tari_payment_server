//! # Order memo signature format
//!
//! When providing a Tari address in a Shopify order, we cannot let users provide just any wallet address there,
//! because this would let folks put other wallet addresses in there and hope that one day someone makes a payment
//! from that wallet and their order will then be fulfilled.
//!
//! Users need to _prove_ that they own the wallet address they provide in the order. This is done by signing a message
//! with the wallet's private key. The message is constructed from the wallet address and the order ID (preventing
//! naughty people from using the same signature for their own orders, and again, trying to get free stuff).
//!
//! The signature is then stored in the order memo field, and the payment server can verify the signature by checking
//! the wallet's public key against the signature.
//!
//! ## Message format
//!
//! The message is constructed by concatenating the wallet address and the order ID, separated by a colon.
//! The challenge is a domain-separated Schnorr signature. The full format is:
//!
//! ```text
//!    {aaaaaaaa}MemoSignature.v1.challenge{bbbbbbbb}{address}:{order_id}
//! ```
//!
//! where
//!   * `aaaaaaaa` is the length of `MemoSignature.v1.challenge`, i.e. 25 in little-endian format.
//!   * `bbbbbbbb` is the length of `address`(64) + `:`(1) + `order_id.len()` in little-endian format.
//!   * `address` is the Tari address of the wallet owner, in hexadecimal
//!   * `order_id` is the order ID, a string
//!
//! The message is then hashed with `Blake2b<U64>` to get the challenge.

use serde::{Deserialize, Serialize};
use tari_common_types::tari_address::TariAddress;
use tari_crypto::{
    hash_domain,
    hashing::DomainSeparation,
    ristretto::{RistrettoPublicKey, RistrettoSchnorrWithDomain, RistrettoSecretKey},
    signatures::SchnorrSignatureError,
    tari_utilities::hex::Hex,
};
use thiserror::Error;

use crate::db_types::{NewOrder, SerializedTariAddress};

hash_domain!(MemoSignatureDomain, "MemoSignature");

pub type MemoSchnorr = RistrettoSchnorrWithDomain<MemoSignatureDomain>;

#[derive(Debug, Clone, Error)]
#[error("Invalid memo signature: {0}")]
pub struct MemoSignatureError(String);

impl From<String> for MemoSignatureError {
    fn from(e: String) -> Self {
        Self(e)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoSignature {
    pub address: SerializedTariAddress,
    pub order_id: String,
    #[serde(serialize_with = "ser_sig", deserialize_with = "de_sig")]
    pub signature: MemoSchnorr,
}

impl MemoSignature {
    pub fn create(
        address: TariAddress,
        order_id: String,
        secret_key: &RistrettoSecretKey,
    ) -> Result<Self, MemoSignatureError> {
        let address = SerializedTariAddress::from(address);
        let message = signature_message(&address, &order_id);
        let signature = sign_message(&message, secret_key).map_err(|e| MemoSignatureError(e.to_string()))?;
        Ok(Self { address, order_id, signature })
    }

    pub fn new(address: &str, order_id: &str, signature: &str) -> Result<Self, MemoSignatureError> {
        let address = address.parse::<SerializedTariAddress>().map_err(|e| MemoSignatureError(e.to_string()))?;
        let signature = hex_to_schnorr::<_, MemoSignatureError>(signature)?;
        let order_id = order_id.to_string();
        Ok(Self { address, order_id, signature })
    }

    pub fn message(&self) -> String {
        signature_message(&self.address, &self.order_id)
    }

    pub fn is_valid(&self) -> bool {
        let message = self.message();
        let pubkey = self.address.as_address().public_key();
        self.signature.verify(pubkey, message)
    }

    pub fn as_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

pub fn signature_message(address: &SerializedTariAddress, order_id: &str) -> String {
    let addr = address.as_address().to_hex();
    format!("{addr}:{order_id}")
}

pub fn sign_message(message: &str, secret_key: &RistrettoSecretKey) -> Result<MemoSchnorr, SchnorrSignatureError> {
    let mut rng = rand::thread_rng();
    MemoSchnorr::sign(secret_key, message.as_bytes(), &mut rng)
}

pub fn ser_sig<S>(sig: &MemoSchnorr, s: S) -> Result<S::Ok, S::Error>
where S: serde::Serializer {
    let nonce = sig.get_public_nonce().to_hex();
    let sig = sig.get_signature().to_hex();
    s.serialize_str(&format!("{nonce}{sig}"))
}

pub fn de_sig<'de, D>(d: D) -> Result<MemoSchnorr, D::Error>
where D: serde::Deserializer<'de> {
    let s = String::deserialize(d)?;
    hex_to_schnorr::<_, String>(&s).map_err(|s| serde::de::Error::custom(s))
}

pub fn hex_to_schnorr<H: DomainSeparation, E: From<String>>(s: &str) -> Result<RistrettoSchnorrWithDomain<H>, E> {
    if s.len() != 128 {
        return Err(E::from("Invalid signature length".into()));
    }
    let nonce = RistrettoPublicKey::from_hex(&s[..64])
        .map_err(|e| E::from(format!("Signature contains an invalid public nonce. {e}")))?;
    let sig = RistrettoSecretKey::from_hex(&s[64..])
        .map_err(|e| E::from(format!("Signature contains an invalid signature key. {e}")))?;
    Ok(RistrettoSchnorrWithDomain::new(nonce, sig))
}

pub fn extract_and_verify_memo_signature(order: &NewOrder) -> Result<MemoSignature, MemoSignatureError> {
    let json = order.memo.as_ref().ok_or_else(|| MemoSignatureError("Memo signature is missing".into()))?;
    let sig = serde_json::from_str::<MemoSignature>(json)
        .map_err(|e| MemoSignatureError(format!("Failed to deserialize memo signature. {e}")))?;
    if sig.order_id.as_str() != order.order_id.as_str() {
        return Err(MemoSignatureError("Order ID in memo signature does not match order ID".into()));
    }
    if sig.is_valid() {
        Ok(sig)
    } else {
        Err(MemoSignatureError("Memo object was valid, but signature was invalid".into()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // These tests use this address
    //     ----------------------------- Tari Address -----------------------------
    //     Network: mainnet
    //     Secret key: 1dbbce83de2b0233c404b96b9234233bb3cec51503e2124d8c728a2d9b4fb00c
    //     Public key: a8d523755de41b9c14de709ca59d52bc1772658258962ef5bbefa8c59082e547
    //     Address: a8d523755de41b9c14de709ca59d52bc1772658258962ef5bbefa8c59082e54733
    //     Emoji ID: 👽🔥🍓🐗🎼😉🍊👘🍁🔮🐎👘👣👙🎮💨🍆🐑🏉🐬🎷👒🍪🚜💦🚌👽💼🐼🐬😍🎡🍰
    // ------------------------------------------------------------------------

    fn secret_key() -> RistrettoSecretKey {
        RistrettoSecretKey::from_hex("1dbbce83de2b0233c404b96b9234233bb3cec51503e2124d8c728a2d9b4fb00c").unwrap()
    }

    #[test]
    fn create_memo_signature() {
        let address = "a8d523755de41b9c14de709ca59d52bc1772658258962ef5bbefa8c59082e54733"
            .parse()
            .expect("Failed to parse TariAddress");
        let sig =
            MemoSignature::create(address, "oid554432".into(), &secret_key()).expect("Failed to create memo signature");
        let msg = signature_message(&sig.address, &sig.order_id);
        assert_eq!(msg, "a8d523755de41b9c14de709ca59d52bc1772658258962ef5bbefa8c59082e54733:oid554432");
        assert_eq!(
            sig.address.as_address().to_hex(),
            "a8d523755de41b9c14de709ca59d52bc1772658258962ef5bbefa8c59082e54733"
        );
        assert_eq!(sig.order_id, "oid554432");
        assert!(sig.is_valid());
    }

    #[test]
    fn verify_from_json() {
        let json = r#"{
          "address": "a8d523755de41b9c14de709ca59d52bc1772658258962ef5bbefa8c59082e54733",
          "order_id": "oid554432",
          "signature": "2421e3c98522d7c5518f55ddb39f759ee9051dde8060679d48f257994372fb214e9024917a5befacb132fc9979527ff92daa2c5d42062b8a507dc4e3b6954c05"
        }"#;
        let sig = serde_json::from_str::<MemoSignature>(json).expect("Failed to deserialize memo signature");
        assert_eq!(
            sig.address.as_address().to_hex(),
            "a8d523755de41b9c14de709ca59d52bc1772658258962ef5bbefa8c59082e54733"
        );
        assert_eq!(sig.order_id, "oid554432");
        assert!(sig.is_valid());
    }
}
