use std::env;

use tari_payment_server::config::OrderIdField;

/// Examines the environment configuration to determine the field used to determine the order id.
pub fn order_id_field_from_env() -> OrderIdField {
    env::var("TPG_ORDER_ID_FIELD")
        .map(|s| if s.to_lowercase().as_str() == "name" { OrderIdField::Name } else { OrderIdField::Id })
        .unwrap_or_default()
}
