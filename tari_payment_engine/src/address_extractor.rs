use crate::db_types::OrderId;
use log::{error, warn};
use tari_common_types::tari_address::TariAddress;

pub fn extract_public_key_from_memo(memo: &str) -> Option<TariAddress> {
    error!("Add in a signature check here to avoid order hijacking");
    // Search for an emoji id in the memo
    let hex_address = regex::Regex::new(r"[a-zA-Z0-9]{66}").unwrap();
    let emoji_id = regex::Regex::new(r"(\p{Emoji}){33}").unwrap();
    hex_address.find(memo).and_then(|m| {
        let s = m.as_str();
        match TariAddress::from_hex(s) {
            Err(e) => {
                warn!("We found a hex address in the memo, {s}, but it was not a valid TariAddress: {e}");
                None
            },
            Ok(addr) => Some(addr)
        }
    }).or_else(|| {
        emoji_id.find(memo).and_then(|m| {
            let s = m.as_str();
            match TariAddress::from_emoji_string(s) {
                Err(e) => {
                    warn!("We found an emoji id in the memo, {s}, but it was not a valid TariAddress: {e}");
                    None
                },
                Ok(addr) => Some(addr)
            }
        })
    })
}

pub fn extract_order_number_from_memo(memo: &str) -> Option<OrderId> {
    let order_number = regex::Regex::new(r"\[([\d\w]+)\]").unwrap();
    order_number
        .captures(memo)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string().into()))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn find_hex_public_keys() {
        let pk = extract_public_key_from_memo("");
        assert_eq!(pk, None);
        let pk = extract_public_key_from_memo("Some random test");
        assert_eq!(pk, None);
        // 33 hex characters, but an invalid address
        let pk = extract_public_key_from_memo(
            "AbCdEf010203040506070809A0a1a2a3AbCdEf010203040506070809A0a1a2a3a4",
        );
        assert_eq!(pk, None);
        // Finds a valid hex pubkey and normalises it
        let pk = extract_public_key_from_memo(
            "Pubkey: 28974F5F2A5FBE2F470CD971AE86E5AD86EFA0844F9123E7FC2E56C8EFA03D0221. Joe Po",
        )
        .unwrap();
        assert_eq!(
            pk.to_hex(),
            "28974f5f2a5fbe2f470cd971ae86e5ad86efa0844f9123e7fc2e56c8efa03d0221"
        );
    }

    #[test]
    fn find_emoji_ids() {
        // Too short
        let pk = extract_public_key_from_memo(
            "address: 🏦🍟🐸🏁🔭📝👠🍈🌻🐚🍞🐓🌝👞🐢📌🍔🎤🚨🍣🐀😿💸💡🎁😉🍉🎃🐳🌷🎢👓",
        );
        assert_eq!(pk, None);
        let pk = extract_public_key_from_memo(
            "address: 🏦🍟🎵🐸🏁🔭📝👠🍈🌻🐚🍞🐓🌝👞🐢📌🍔🎤🚨🍣🐀😿💸💡🎁😉🍉🎃🐳🌷🎢👓.",
        )
        .unwrap();
        let expected = TariAddress::from_hex(
            "6829578d62ddcba2191178287307a07dc8244af92b6bebc2b83ee41a40880e4897",
        )
        .unwrap();
        assert_eq!(pk, expected);
    }

    #[test]
    fn find_order_numbers() {
        let order = extract_order_number_from_memo("");
        assert_eq!(order, None);
        let order = extract_order_number_from_memo("Some random test");
        assert_eq!(order, None);
        let order = extract_order_number_from_memo("[1234]").unwrap();
        assert_eq!(order.as_str(), "1234");
        let order = extract_order_number_from_memo("Order#: [Some Order Number]");
        assert_eq!(order, None);
        let order = extract_order_number_from_memo("Order#: [SomeOrderNumber]").unwrap();
        assert_eq!(order.as_str(), "SomeOrderNumber");
    }
}
