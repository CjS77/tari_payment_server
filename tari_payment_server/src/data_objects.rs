use std::fmt::Display;

use serde::{Deserialize, Serialize};
use tari_payment_engine::{
    db_types::{NewPayment, OrderId, Role, SerializedTariAddress},
    helpers::WalletSignature,
    tpe_api::exchange_objects::ExchangeRate,
};
use tpg_common::MicroTari;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveOrderParams {
    pub order_id: OrderId,
    pub new_customer_id: String,
    // This reason is not stored in the database, but is captured in the logs
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachOrderParams {
    pub order_id: OrderId,
    pub address: SerializedTariAddress,
    // This reason is not stored in the database, but is captured in the logs
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateUpdate {
    pub currency: String,
    pub rate: u64,
}

impl From<ExchangeRateUpdate> for ExchangeRate {
    fn from(update: ExchangeRateUpdate) -> Self {
        #[allow(clippy::cast_possible_wrap)]
        Self::new(update.currency, update.rate as i64, None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateResult {
    pub currency: String,
    pub rate: i64,
    pub updated_at: String,
}

impl From<ExchangeRate> for ExchangeRateResult {
    fn from(rate: ExchangeRate) -> Self {
        Self { currency: rate.base_currency, rate: rate.rate / 100, updated_at: rate.updated_at.to_rfc3339() }
    }
}
