use log::*;
use serde_json::Value;

pub fn json_is_subset_of(part: &str, complete: &str) -> bool {
    let part: Value = serde_json::from_str(part).expect("Invalid JSON");
    let complete: Value = serde_json::from_str(complete).expect("Invalid JSON");
    value_is_subset_of(&part, &complete)
}

pub fn value_is_subset_of(part: &Value, complete: &Value) -> bool {
    if part.is_null() {
        debug!("given value is null, which is always a subset of any value");
        return true;
    }
    if part.is_object() {
        for (key, value) in part.as_object().expect("Not an object") {
            match complete.get(key) {
                Some(complete_value) => {
                    if !value_is_subset_of(value, complete_value) {
                        error!("Value mismatch: {} != {}", value, complete_value);
                        return false;
                    }
                },
                None => {
                    error!("Key not found: {}", key);
                    return false;
                },
            }
        }
        true
    } else if part.is_array() {
        if !complete.is_array() {
            error!("Given object is an array, but we do not expect an array");
            return false;
        }
        let arr_p = part.as_array().expect("Not an array");
        let arr_c = complete.as_array().expect("Not an array");
        if arr_p.len() != arr_c.len() {
            error!("Array length mismatch: {} != {}", arr_p.len(), arr_c.len());
            return false;
        }
        arr_p.iter().zip(arr_c.iter()).all(|(p, c)| value_is_subset_of(p, c))
    } else {
        part == complete
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple_json_is_subset() {
        let part = r#"1"#;
        let complete = r#"2"#;
        assert!(super::json_is_subset_of(part, part));
        assert!(!super::json_is_subset_of(part, complete));
    }

    #[test]
    fn json_is_subset_nulls() {
        let part = Value::Null;
        let complete = Value::Bool(true);
        assert!(value_is_subset_of(&part, &part));
        assert!(value_is_subset_of(&part, &complete));
    }

    #[test]
    fn is_subset_array() {
        let part = r#"[1, 2, 3]"#;
        let complete = r#"[1, 2, 3, 4]"#;
        assert!(json_is_subset_of(part, part));
        assert!(!json_is_subset_of(part, complete));
    }

    #[test]
    fn is_subset_simple_object() {
        let part = r#"{"a": 1, "b": 2}"#;
        let complete = r#"{"a": 1, "b": 2, "c": 3}"#;
        assert!(json_is_subset_of(part, part));
        // Not all keys needs to present. Only the ones in part
        assert!(json_is_subset_of(part, complete));
    }

    #[test]
    fn not_subset() {
        let part = r#"{"a": 1, "b": 2}"#;
        let complete = r#"{"a": 1, "b": 2, "c": 3}"#;
        assert!(json_is_subset_of(part, complete));
        assert!(!json_is_subset_of(complete, part));
    }

    #[test]
    fn is_subset_complex_object() {
        let part = r#"{"a": 1, "b": [1,2,3], "c": {"foo": 2, "bar": [] }}"#;
        let complete = r#"{"a": 1, "b": [1,2,3], "b2": null, "c": {"foo": 2, "bar": [], "baz": [1],  "x": 45.2}}"#;
        assert!(json_is_subset_of(part, complete));
        assert!(json_is_subset_of(complete, complete));
    }

    #[test]
    fn is_subset_complex_object_fail() {
        let complete = r#"
        {
          "address":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
          "total_orders":165000000,
          "orders":[
            {"id":1,"order_id":"1","customer_id":"alice",
             "memo":"address: [680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b]",
             "total_price":100000000,
             "currency":"XTR",
             "status":"New"},
            {"id":3,"order_id":"3","customer_id":"alice",
            "memo":"address: [680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b]",
            "total_price":65000000,"currency":"XTR",
            "status":"New"}
          ]}
        "#;
        let part = r#"{ "address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d" }"#;
        assert!(!json_is_subset_of(part, complete));
        let part = r#"{
          "address":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
          "total_orders":165
        }"#;
        assert!(!json_is_subset_of(part, complete));
        let part = r#"
        {
          "address":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
          "total_orders":165000000,
          "orders":[]}
        "#;
        assert!(!json_is_subset_of(part, complete));
    }
}
