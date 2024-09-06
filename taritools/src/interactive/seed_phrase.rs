use std::str::FromStr;

use anyhow::{anyhow, Result};
use blake2::Blake2b;
use digest::consts::U64;
use tari_common_types::{key_branches::DATA_ENCRYPTION, WALLET_COMMS_AND_SPEND_KEY_BRANCH};
use tari_crypto::{
    keys::PublicKey,
    ristretto::{RistrettoPublicKey, RistrettoSecretKey},
};
use tari_key_manager::{cipher_seed::CipherSeed, key_manager::KeyManager, mnemonic::Mnemonic, SeedWords};
use zeroize::Zeroize;

pub fn string_to_seed_words(mut seed_words: String) -> Result<SeedWords> {
    let result = SeedWords::from_str(&seed_words).map_err(|e| anyhow!("SeedWordsError parsing seed words: {}", e))?;
    seed_words.zeroize();
    Ok(result)
}
pub fn seed_words_to_keys(seed_words: SeedWords) -> Result<(RistrettoSecretKey, RistrettoPublicKey)> {
    let seed = CipherSeed::from_mnemonic(&seed_words, None)?;
    drop(seed_words);
    let spend_key_branch = WALLET_COMMS_AND_SPEND_KEY_BRANCH.to_string();
    let spend_key_manager = KeyManager::<RistrettoPublicKey, Blake2b<U64>>::from(seed.clone(), spend_key_branch, 0);
    let spend_key = spend_key_manager.derive_key(0)?.key;
    drop(spend_key_manager);
    let view_key_branch = DATA_ENCRYPTION.to_string();
    let view_key_manager = KeyManager::<RistrettoPublicKey, Blake2b<U64>>::from(seed, view_key_branch, 0);
    let view_key = view_key_manager.derive_key(0)?.key;
    let public_view_key = RistrettoPublicKey::from_secret_key(&view_key);
    Ok((spend_key, public_view_key))
}
