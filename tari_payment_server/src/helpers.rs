use std::{
    fmt,
    fmt::{Debug, Display},
    net::IpAddr,
    str::FromStr,
};

use actix_web::HttpRequest;
use base64::encode;
use hmac::{Hmac, Mac};
use log::{debug, trace};
use regex::Regex;
use sha2::Sha256;

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
    encode(&code_bytes)
}

#[derive(Clone, Default)]
pub struct Secret<T>
where T: Clone + Default
{
    value: T,
}

impl<T: Clone + Default> Secret<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn reveal(&self) -> &T {
        &self.value
    }
}

impl<T: Clone + Default> Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("****")
    }
}

impl<T: Clone + Default> Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("****")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_calculate_hmac() {
        let data = r#"{"id":5621189509332,..."source":"shopify"}"#;
        let hmac = calculate_hmac("my_secret", data.as_bytes());
        assert_eq!(hmac, "1JKXEbaNTtaw8EyjOKhDEJL/hE/SKH1ZZADWGD36m6k=")
    }
}
