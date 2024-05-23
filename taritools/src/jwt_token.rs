use chrono::Utc;
use serde::Serialize;
use tari_common::configuration::Network;
use tari_common_types::tari_address::TariAddress;
use tari_crypto::{
    keys::PublicKey,
    ristretto::{RistrettoPublicKey, RistrettoSecretKey},
    tari_utilities::hex::Hex,
};
use tari_jwt::{
    jwt_compact::{AlgorithmExt, Claims, Header},
    Ristretto256,
    Ristretto256SigningKey,
};

pub fn print_jwt_token(sk: String, network: Network, roles: Vec<String>) {
    let Ok(sk) = RistrettoSecretKey::from_hex(&sk) else {
        println!("Invalid secret key");
        return;
    };
    let pubkey = RistrettoPublicKey::from_secret_key(&sk);
    let address = TariAddress::new(pubkey.clone(), network);
    let nonce = Utc::now().timestamp() as u64;
    let claims = LoginToken { address, nonce, desired_roles: roles };
    let claims = Claims::new(claims);
    let header = Header::empty().with_token_type("JWT");
    let msg = Ristretto256.token(&header, &claims, &Ristretto256SigningKey(sk)).unwrap_or_else(|e| e.to_string());
    println!("----------------------------- Access Token -----------------------------");
    println!("address: {}", claims.custom.address);
    println!("address: {}", claims.custom.address.to_hex());
    println!("network: {}", claims.custom.address.network());
    println!("nonce: {}", claims.custom.nonce);
    println!("roles: {:}", claims.custom.desired_roles.into_iter().collect::<Vec<String>>().join(","));
    println!("token:\n{msg}");
    println!("------------------------------------------------------------------------");
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoginToken {
    pub address: TariAddress,
    pub nonce: u64,
    pub desired_roles: Vec<String>,
}
