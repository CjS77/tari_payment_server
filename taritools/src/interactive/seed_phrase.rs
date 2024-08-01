use std::str::FromStr;

use anyhow::{anyhow, Result};
use blake2::Blake2b;
use digest::consts::U64;
use tari_crypto::ristretto::{RistrettoPublicKey, RistrettoSecretKey};
use tari_key_manager::{cipher_seed::CipherSeed, key_manager::KeyManager, mnemonic::Mnemonic, SeedWords};
use zeroize::Zeroize;

pub fn string_to_seed_words(mut seed_words: String) -> Result<SeedWords> {
    let result = SeedWords::from_str(&seed_words).map_err(|e| anyhow!("SeedWordsError parsing seed words: {}", e))?;
    seed_words.zeroize();
    Ok(result)
}
pub fn seed_words_to_comms_key(seed_words: SeedWords) -> Result<RistrettoSecretKey> {
    let seed = CipherSeed::from_mnemonic(&seed_words, None)?;
    drop(seed_words);
    let comms_key_manager = KeyManager::<RistrettoPublicKey, Blake2b<U64>>::from(seed, "comms".into(), 0);
    let key = comms_key_manager.derive_key(0)?.key;
    Ok(key)
}
