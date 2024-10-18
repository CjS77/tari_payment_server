use std::{net::IpAddr, str::FromStr};

use actix_web::HttpRequest;
use base64::encode;
use hmac::{Hmac, Mac};
use log::{debug, trace};
use regex::Regex;
use sha2::Sha256;
use tari_payment_engine::{
    db_types::{NewPayment, OrderId},
    helpers::{extract_order_id_from_str, MemoSignature},
};

use crate::config::OrderIdField;

/// Get the remote IP address from the request. It uses 3 sources to determine the IP address, in decreasing order
/// of preference:
/// 1. The `X-Forwarded-For` header, iif `use_x_forwarded_for` is set to true in the configuration.
/// 2. The `Forwarded` header, iif `use_forwarded` is set to true in the configuration.
/// 3. The peer address from the connection info.
pub fn get_remote_ip(req: &HttpRequest, use_x_forwarded_for: bool, use_forwarded: bool) -> Option<IpAddr> {
    // Collect peer IP from x-forwarded-for, or forwarded headers _if_ `use_nnn` has been set to true
    // in the configuration. Otherwise, use the peer address from the connection info.
    let mut result = None;
    if use_x_forwarded_for {
        trace!("Checking X-Forwarded-For header");
        result =
            req.headers().get("X-Forwarded-For").and_then(|v| v.to_str().ok()).and_then(|s| IpAddr::from_str(s).ok());
        if let Some(ip) = result {
            debug!("Using X-Forwarded-For header for remote address: {ip}");
        }
    }
    if use_forwarded && result.is_none() {
        trace!("Checking Forwarded header");
        let re = Regex::new(r#"for=(?P<ip>[^;]+)"#).unwrap();
        result = req
            .headers()
            .get("Forwarded")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| re.captures(v))
            .and_then(|caps| caps.name("ip"))
            .map(|m| m.as_str())
            .and_then(|s| IpAddr::from_str(s).ok());
        if let Some(ip) = result {
            debug!("Using Forwarded header for remote address: {ip}");
        }
    }
    // If both use_x_forwarded_for and use_forwarded are set to true, overwrite the result from the Forwarded header
    result.or_else(|| {
        let peer_addr = req.connection_info().peer_addr().map(|a| a.to_string());
        trace!("Using Peer address for remote address: {:?}", peer_addr);
        peer_addr.and_then(|s| IpAddr::from_str(&s).ok())
    })
}

pub fn calculate_hmac(secret: &str, data: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(data);
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    encode(code_bytes)
}

/// Tries to extract the order number from the memo.
///
/// If the memo is not present, return None.
/// If an order is successfully extracted, return `Some(true)`, otherwise `Some(false)`.
///
/// Otherwise, if `require_signature` is true, the memo must contain a valid `MemoSignature` object:
///   1. The memo bust be a valid JSON object.
///   2. The `claim` field must be present.
///   3. The `claim` field must be a valid JSON object containing a valid `MemoSignature`.
///
/// If `require_signature` is false, the first number encountered in the memo is used as the order number.
/// If orderIdField is `OrderIdField::Name`, the number will be prefixed with a `#`.
pub fn try_extract_order_id(
    payment: &mut NewPayment,
    require_signature: bool,
    order_id_field: OrderIdField,
) -> Option<bool> {
    payment.memo.as_ref().map(|m| match serde_json::from_str::<MemoSignature>(m) {
        Ok(m) => {
            let result = m.is_valid();
            if result {
                payment.order_id = Some(OrderId::new(m.order_id));
            }
            result
        },
        Err(_) if require_signature => false,
        Err(_) => {
            let prefix = match order_id_field {
                OrderIdField::Id => "",
                OrderIdField::Name => "#",
            };
            if let Some(order_id) = extract_order_id_from_str(m, prefix) {
                payment.order_id = Some(order_id);
                true
            } else {
                false
            }
        },
    })
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use tari_common_types::tari_address::TariAddress;
    use tpg_common::MicroTari;

    use super::*;

    #[test]
    fn test_calculate_hmac() {
        let data = r#"{"id":5621189509332,..."source":"shopify"}"#;
        let hmac = calculate_hmac("my_secret", data.as_bytes());
        assert_eq!(hmac, "1JKXEbaNTtaw8EyjOKhDEJL/hE/SKH1ZZADWGD36m6k=")
    }

    #[test]
    fn extract_order_id_with_signature() {
        let mut payment = NewPayment::new(
            TariAddress::from_str("14s9vDTwrweZvWEgQ9gNhXXPX68DPXSSAHNFWYEPi5JsBQY").unwrap(),
            MicroTari::from_tari(100),
            "txid111111".to_string(),
        );
        payment.with_memo(json!({
            "address": "14s9vDTwrweZvWEgQ9gNhXXPX68DPXSSAHNFWYEPi5JsBQY",
            "order_id": "oid554432",
            "signature": "74236918f5815383ad7a889fa2c26037418b217f983575b5b5cfde21c7bcf3094ca6ff09c43fca8d4040a38e60b57fea622d5919979fae4ccfea93883df6bd00"
            }).to_string()
        );
        let result = try_extract_order_id(&mut payment, true, OrderIdField::Id);
        assert!(matches!(result, Some(true)));
        assert_eq!(payment.order_id.unwrap().as_str(), "oid554432");
    }

    #[test]
    fn extract_order_id_with_invalid_signature() {
        let mut payment = NewPayment::new(
            TariAddress::from_str("14s9vDTwrweZvWEgQ9gNhXXPX68DPXSSAHNFWYEPi5JsBQY").unwrap(),
            MicroTari::from_tari(100),
            "txid111111".to_string(),
        );
        payment.with_memo(json!({
            "address": "14s9vDTwrweZvWEgQ9gNhXXPX68DPXSSAHNFWYEPi5JsBQY",
            "order_id": "oid554432",
            "signature": "00000018f5815383ad7a889fa2c26037418b217f983575b5b5cfde21c7bcf3094ca6ff09c43fca8d4040a38e60b57fea622d5919979fae4ccfea93883df6bd00"
            }).to_string()
        );
        let result = try_extract_order_id(&mut payment, true, OrderIdField::Id);
        assert!(matches!(result, Some(false)));
        assert!(payment.order_id.is_none());
    }

    #[test]
    fn extract_order_id_with_raw_memo() {
        let mut payment = NewPayment::new(
            TariAddress::from_str("14s9vDTwrweZvWEgQ9gNhXXPX68DPXSSAHNFWYEPi5JsBQY").unwrap(),
            MicroTari::from_tari(100),
            "txid111111".to_string(),
        );
        payment.with_memo("order #12345");
        let result = try_extract_order_id(&mut payment, false, OrderIdField::Name);
        assert!(matches!(result, Some(true)));
        assert_eq!(payment.order_id.as_ref().unwrap().as_str(), "#12345");
        let result = try_extract_order_id(&mut payment, false, OrderIdField::Id);
        assert!(matches!(result, Some(true)));
        assert_eq!(payment.order_id.unwrap().as_str(), "12345");
    }
}
