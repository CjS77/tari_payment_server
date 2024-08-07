use std::{
    fmt::Display,
    hash::{Hash, Hasher},
    str::FromStr,
};

use chrono::{DateTime, Utc};
use log::{error, trace};
use serde::{Deserialize, Serialize};
use sqlx::{database::HasValueRef, Database, Decode, FromRow, Sqlite, Type};
use tari_common_types::tari_address::{TariAddress, TariAddressError};
use thiserror::Error;
use tpg_common::MicroTari;

use crate::{
    helpers::{extract_and_verify_memo_signature, MemoSignature, MemoSignatureError},
    tpe_api::order_objects::{address_to_hex, str_to_address},
};

//--------------------------------------     PublicKey       ---------------------------------------------------------
/// A lightweight wrapper around a string representing a public key
#[derive(Clone, Debug, Type, PartialEq, Eq)]
#[sqlx(transparent)]
pub struct PublicKey(pub String);

impl Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<S: Into<String>> From<S> for PublicKey {
    fn from(value: S) -> Self {
        Self(value.into())
    }
}

//--------------------------------------     TariAddress       ---------------------------------------------------------
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SerializedTariAddress(
    #[serde(serialize_with = "address_to_hex", deserialize_with = "str_to_address")] TariAddress,
);

impl SerializedTariAddress {
    pub fn to_address(self) -> TariAddress {
        self.0
    }

    pub fn as_address(&self) -> &TariAddress {
        &self.0
    }

    pub fn as_hex(&self) -> String {
        self.0.to_hex()
    }
}

impl AsRef<TariAddress> for SerializedTariAddress {
    fn as_ref(&self) -> &TariAddress {
        &self.0
    }
}

impl FromStr for SerializedTariAddress {
    type Err = TariAddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<TariAddress>().map(Self)
    }
}

impl TryFrom<String> for SerializedTariAddress {
    type Error = TariAddressError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse::<TariAddress>().map(Self)
    }
}

impl From<TariAddress> for SerializedTariAddress {
    fn from(value: TariAddress) -> Self {
        Self(value)
    }
}

impl From<&TariAddress> for SerializedTariAddress {
    fn from(value: &TariAddress) -> Self {
        Self(value.clone())
    }
}

impl Display for SerializedTariAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_hex())
    }
}

impl<'r, DB: Database> Decode<'r, DB> for SerializedTariAddress
// we want to delegate some of the work to string
// decoding so let's make sure strings are supported by
// the database
where &'r str: Decode<'r, DB>
{
    fn decode(
        value: <DB as HasValueRef<'r>>::ValueRef,
    ) -> Result<SerializedTariAddress, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let value = <&str as Decode<DB>>::decode(value)?;
        let addr = value.parse::<TariAddress>()?;
        Ok(addr.into())
    }
}

impl Type<Sqlite> for SerializedTariAddress {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl PartialEq for SerializedTariAddress {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for SerializedTariAddress {}

impl Hash for SerializedTariAddress {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_hex().hash(state);
    }
}

//--------------------------------------     UserAccount       ---------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, Default)]
pub struct UserAccount {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_received: MicroTari,
    pub current_pending: MicroTari,
    pub current_balance: MicroTari,
    pub total_orders: MicroTari,
    pub current_orders: MicroTari,
}

//--------------------------------------   OrderStatusType     ---------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
pub enum OrderStatusType {
    /// The order has been created and the payment has been received in full
    Paid,
    /// The order has been cancelled by the user or admin.
    Cancelled,
    /// The order has expired.
    Expired,
    /// The order is newly created, unpaid, and matched to a wallet address
    New,
    /// The order is newly created, and is not associated with any wallet address
    Unclaimed,
}

impl Display for OrderStatusType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderStatusType::Paid => write!(f, "Paid"),
            OrderStatusType::Cancelled => write!(f, "Cancelled"),
            OrderStatusType::Expired => write!(f, "Expired"),
            OrderStatusType::New => write!(f, "New"),
            OrderStatusType::Unclaimed => write!(f, "Unclaimed"),
        }
    }
}

impl From<String> for OrderStatusType {
    fn from(value: String) -> Self {
        value.parse().unwrap_or_else(|_| {
            error!("Invalid order status: {value}. But this conversion cannot fail. Defaulting to Unclaimed");
            OrderStatusType::Unclaimed
        })
    }
}

#[derive(Debug, Clone, Error)]
#[error("Invalid conversion from string: {0}")]
pub struct ConversionError(String);

impl FromStr for OrderStatusType {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Paid" => Ok(Self::Paid),
            "Cancelled" => Ok(Self::Cancelled),
            "Expired" => Ok(Self::Expired),
            "New" => Ok(Self::New),
            "Unclaimed" => Ok(Self::Unclaimed),
            s => Err(ConversionError(format!("Invalid order status: {s}"))),
        }
    }
}

//--------------------------------------        OrderId        ---------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct OrderId(pub String);

impl FromStr for OrderId {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl From<String> for OrderId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl OrderId {
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

//--------------------------------------     OrderStatus       ---------------------------------------------------------
#[derive(Debug, Clone)]
pub struct OrderStatus {
    pub id: i64,
    pub order_id: OrderId,
    pub updated_at: DateTime<Utc>,
    pub status: OrderStatusType,
}

//--------------------------------------        Order       ---------------------------------------------------------
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Order {
    pub id: i64,
    pub order_id: OrderId,
    pub customer_id: String,
    pub memo: Option<String>,
    pub total_price: MicroTari,
    pub original_price: Option<String>,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: OrderStatusType,
}

impl PartialEq for Order {
    fn eq(&self, other: &Self) -> bool {
        self.order_id == other.order_id &&
            self.customer_id == other.customer_id &&
            self.total_price == other.total_price &&
            self.currency == other.currency
    }
}

impl Eq for Order {}

impl Order {
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        match self.status {
            OrderStatusType::New => Some(self.updated_at + chrono::Duration::hours(6)),
            // todo! add claimed status to extend expiry time
            _ => None,
        }
    }
}

//--------------------------------------        NewOrder       ---------------------------------------------------------
#[derive(Debug, Clone)]
pub struct NewOrder {
    /// The order_id as assigned by Shopify
    pub order_id: OrderId,
    /// The customer_id as assigned by Shopify
    pub customer_id: String,
    /// An optional description supplied by the user for the order. Useful for matching orders with payments
    pub memo: Option<String>,
    /// The public key of the user who made the payment, usually extracted from the memo
    pub address: Option<TariAddress>,
    /// The total price of the order
    pub total_price: MicroTari,
    /// The original price of the order, in `currency` units
    pub original_price: Option<String>,
    /// The currency of the order
    pub currency: String,
    /// The time the order was created on Shopify
    pub created_at: DateTime<Utc>,
}

impl NewOrder {
    pub fn new(order_id: OrderId, customer_id: String, total_price: MicroTari) -> Self {
        Self {
            order_id,
            customer_id,
            memo: None,
            total_price,
            original_price: None,
            currency: "XTR".to_string(),
            created_at: Utc::now(),
            address: None,
        }
    }

    /// Tries to extract the address from the memo
    pub fn try_extract_address(&mut self) -> Result<(), MemoSignatureError> {
        let sig = extract_and_verify_memo_signature(self)?;
        trace!("Extracted address from memo and confirmed signature was correct");
        self.address = Some(sig.address.to_address());
        Ok(())
    }

    pub fn is_equivalent(&self, order: &Order) -> bool {
        self.order_id == order.order_id &&
            self.customer_id == order.customer_id &&
            self.memo == order.memo &&
            self.total_price == order.total_price &&
            self.currency == order.currency &&
            self.created_at == order.created_at
    }
}

impl Display for NewOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Order #{order_id} @ \"{customer_id}\". {total_price} ({created_at})",
            order_id = self.order_id,
            customer_id = self.customer_id,
            total_price = self.total_price,
            created_at = self.created_at
        )
    }
}

//--------------------------------------        Payment       ---------------------------------------------------------
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct Payment {
    pub txid: String,
    /// The time the payment was received
    pub created_at: DateTime<Utc>,
    /// The time the payment was updated
    pub updated_at: DateTime<Utc>,
    /// The public key of the user who made the payment
    pub sender: SerializedTariAddress,
    /// The amount of the payment
    pub amount: MicroTari,
    /// The memo attached to the transfer
    pub memo: Option<String>,
    /// The customer id associated with this order. Generally, this is extracted from the memo.
    pub order_id: Option<OrderId>,
    pub payment_type: PaymentType,
    pub status: TransferStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PaymentType {
    OnChain,
    Manual,
}

impl Default for PaymentType {
    fn default() -> Self {
        Self::OnChain
    }
}

impl From<String> for PaymentType {
    fn from(value: String) -> Self {
        value.as_str().parse().unwrap_or_else(|e| panic!("Invalid payment type: {}. {e}", value))
    }
}

impl FromStr for PaymentType {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OnChain" => Ok(Self::OnChain),
            "Manual" => Ok(Self::Manual),
            s => Err(ConversionError(format!("Invalid payment type: {s}"))),
        }
    }
}

impl Display for PaymentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentType::OnChain => write!(f, "OnChain"),
            PaymentType::Manual => write!(f, "Manual"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPayment {
    /// The public key of the user who made the payment
    pub sender: SerializedTariAddress,
    /// The amount of the payment
    pub amount: MicroTari,
    /// The transaction identifier. Typically, the kernel signature in Tari
    pub txid: String,
    /// The memo attached to the transfer
    pub memo: Option<String>,
    /// The order number associated with this payment. Generally extracted from the memo.
    pub order_id: Option<OrderId>,
}

impl NewPayment {
    pub fn new(sender: TariAddress, amount: MicroTari, txid: String) -> Self {
        Self { sender: sender.into(), amount, txid, memo: None, order_id: None }
    }

    pub fn with_memo<S: Into<String>>(&mut self, memo: S) {
        self.memo = Some(memo.into());
    }

    /// Tries to extract the order number from the memo.
    ///
    /// For this to succeed,
    /// 1. The memo bust be a valid JSON object.
    /// 2. The `claim` field must be present.
    /// 3. The `claim` field must be a valid JSON object containing a valid `MemoSignature`.
    pub fn try_extract_order_id(&mut self) -> Option<bool> {
        self.memo.as_ref().map(|m| match serde_json::from_str::<MemoSignature>(m) {
            Ok(m) => {
                let result = m.is_valid();
                if result {
                    self.order_id = Some(OrderId::new(m.order_id));
                }
                result
            },
            Err(_) => false,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditNote {
    pub customer_id: String,
    /// The amount to credit the user
    pub amount: MicroTari,
    /// The reason for the credit note
    pub reason: Option<String>,
}

impl CreditNote {
    pub fn new(customer_id: String, amount: MicroTari) -> Self {
        Self { customer_id, amount, reason: None }
    }

    pub fn with_reason<S: Into<String>>(mut self, reason: S) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

//-----------------------------------------   PaymentStatus   ---------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    Received,
    Confirmed,
    Cancelled,
}

impl Default for TransferStatus {
    fn default() -> Self {
        Self::Received
    }
}

impl Display for TransferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferStatus::Received => write!(f, "Received"),
            TransferStatus::Confirmed => write!(f, "Confirmed"),
            TransferStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl From<String> for TransferStatus {
    fn from(value: String) -> Self {
        value.as_str().parse().unwrap_or_else(|e| panic!("Invalid transfer status: {value}. {e}"))
    }
}

impl FromStr for TransferStatus {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Received" => Ok(Self::Received),
            "Confirmed" => Ok(Self::Confirmed),
            "Cancelled" => Ok(Self::Cancelled),
            s => Err(ConversionError(format!("Invalid transfer status: {s}"))),
        }
    }
}

//--------------------------------------        User roles       ------------------------------------------------------

pub type Roles = Vec<Role>;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    ReadAll,
    Write,
    User,
    // Allows the address to access payment notification endpoints
    PaymentWallet,
    // Give access to very sensitive operations, such as adding new payment wallets.
    SuperAdmin,
}

impl FromStr for Role {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read_all" => Ok(Self::ReadAll),
            "write" => Ok(Self::Write),
            "user" => Ok(Self::User),
            "payment_wallet" => Ok(Self::PaymentWallet),
            "super_admin" => Ok(Self::SuperAdmin),
            s => Err(ConversionError(format!("Invalid role: {s}"))),
        }
    }
}

pub fn admin() -> Roles {
    vec![Role::ReadAll, Role::Write, Role::User]
}

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::ReadAll => write!(f, "read_all"),
            Role::Write => write!(f, "write"),
            Role::User => write!(f, "user"),
            Role::PaymentWallet => write!(f, "payment_wallet"),
            Role::SuperAdmin => write!(f, "super_admin"),
        }
    }
}

//--------------------------------------    Authentication  ---------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginToken {
    pub address: TariAddress,
    pub nonce: u64,
    pub desired_roles: Roles,
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    #[test]
    fn extract_order_id() {
        let mut payment = NewPayment::new(
            TariAddress::from_str("a8d523755de41b9c14de709ca59d52bc1772658258962ef5bbefa8c59082e54733").unwrap(),
            MicroTari::from_tari(100),
            "txid111111".to_string(),
        );
        payment.with_memo(json!({
            "address": "a8d523755de41b9c14de709ca59d52bc1772658258962ef5bbefa8c59082e54733",
            "order_id": "oid554432",
            "signature": "2421e3c98522d7c5518f55ddb39f759ee9051dde8060679d48f257994372fb214e9024917a5befacb132fc9979527ff92daa2c5d42062b8a507dc4e3b6954c05"
            }).to_string()
        );
        let result = payment.try_extract_order_id();
        assert!(matches!(result, Some(true)));
        assert_eq!(payment.order_id.unwrap().as_str(), "oid554432");
    }
}
