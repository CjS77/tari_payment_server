use crate::db_types::OrderId;

pub fn extract_order_number_from_memo(memo: &str) -> Option<OrderId> {
    let order_number = regex::Regex::new(r"\[([\d\w]+)\]").unwrap();
    order_number.captures(memo).and_then(|c| c.get(1).map(|m| m.as_str().to_string().into()))
}

#[cfg(test)]
mod test {
    use super::*;

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
