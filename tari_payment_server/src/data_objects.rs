use std::fmt::Display;

use serde::{Deserialize, Serialize};
use tari_payment_engine::{
    db_types::{MicroTari, NewPayment, OrderId, Role},
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionConfirmationNotification {
    pub confirmation: TransactionConfirmation,
    pub auth: WalletSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionConfirmation {
    pub txid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyOrderParams {
    pub order_id: OrderId,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMemoParams {
    pub order_id: OrderId,
    pub new_memo: String,
    // This reason is not stored in the database, but is captured in the logs
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePriceParams {
    pub order_id: OrderId,
    pub new_price: MicroTari,
    // This reason is not stored in the database, but is captured in the logs
    pub reason: Option<String>,
}
