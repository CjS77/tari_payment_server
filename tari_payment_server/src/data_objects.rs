use std::fmt::Display;

use serde::{Deserialize, Serialize};
use tari_payment_engine::{
    db_types::{NewPayment, Role},
    helpers::WalletSignature,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleUpdateRequest {
    pub address: String,
    #[serde(default)]
    pub apply: Vec<Role>,
    #[serde(default)]
    pub revoke: Vec<Role>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResponse {
    pub success: bool,
    pub message: String,
}

impl JsonResponse {
    pub fn success<S: Display>(message: S) -> Self {
        Self { success: true, message: message.to_string() }
    }

    pub fn failure<S: Display>(message: S) -> Self {
        Self { success: false, message: message.to_string() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentNotification {
    pub payment: NewPayment,
    pub auth: WalletSignature,
}
