/// Parse a boolean flag from a string value, or return the given default value otherwise.
pub fn parse_boolean_flag(value: Option<String>, default: bool) -> bool {
    let value = match value {
        Some(v) => v,
        None => return default,
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}
