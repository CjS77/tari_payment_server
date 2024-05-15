use std::{
    fmt::Display,
    ops::{Add, Neg, Sub, SubAssign},
    str::FromStr,
};

use chrono::{DateTime, Utc};
use log::error;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use tari_common_types::tari_address::TariAddress;
use thiserror::Error;

use crate::op;

//--------------------------------------     MicroTari       ---------------------------------------------------------
#[derive(Debug, Clone, Copy, Default, Type, Ord, PartialOrd, Serialize, Deserialize)]
#[sqlx(transparent)]
pub struct MicroTari(i64);

pub const T: MicroTari = MicroTari(1_000_000);

op!(binary MicroTari, Add, add);
op!(binary MicroTari, Sub, sub);
op!(inplace MicroTari, SubAssign, sub_assign);
op!(unary MicroTari, Neg, neg);

#[derive(Debug, Clone, Error)]
#[error("Value cannot be represented in microTari: {0}")]
pub struct MicroTariConversionError(String);

impl From<i64> for MicroTari {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl PartialEq for MicroTari {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for MicroTari {}

impl TryFrom<u64> for MicroTari {
    type Error = MicroTariConversionError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value > i64::MAX as u64 {
            Err(MicroTariConversionError(format!("Value {} is too large to convert to MicroTari", value)))
        } else {
            #[allow(clippy::cast_possible_wrap)]
            Ok(Self(value as i64))
        }
    }
}

impl Display for MicroTari {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tari = self.0 as f64 / 1_000_000.0;
        write!(f, "{tari:0.3}τ")
    }
}

impl MicroTari {
    pub fn value(&self) -> i64 {
        self.0
    }

    pub fn from_tari(tari: i64) -> Self {
        Self(tari * 1_000_000)
    }
}

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

//--------------------------------------     UserAccount       ---------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserAccount {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_received: MicroTari,
    pub total_pending: MicroTari,
    pub current_balance: MicroTari,
    pub total_orders: MicroTari,
}

//-------------------------------------- UserAccountPublicKey --------------------------------------------------------
#[derive(Debug, Clone)]
pub struct UserAccountPublicKey {
    pub id: i64,
    pub user_account_id: String,
    pub public_key: PublicKey,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

//-------------------------------------- UserAccountCustomerId --------------------------------------------------------

#[derive(Debug, Clone)]
pub struct UserAccountCustomerId {
    pub id: i64,
    pub user_account_id: String,
    pub customer_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

//--------------------------------------   OrderStatusType     ---------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Type, Serialize, Deserialize)]
pub enum OrderStatusType {
    /// The order has been created and the payment has been received in full
    Paid,
    /// The order has been cancelled by the user or admin.
    Cancelled,
    /// The order has expired.
    Expired,
    /// The order is newly created, and no payments have been received.
    New,
}

impl Display for OrderStatusType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderStatusType::Paid => write!(f, "Paid"),
            OrderStatusType::Cancelled => write!(f, "Cancelled"),
            OrderStatusType::Expired => write!(f, "Expired"),
            OrderStatusType::New => write!(f, "New"),
        }
    }
}

impl From<String> for OrderStatusType {
    fn from(value: String) -> Self {
        value.parse().unwrap_or_else(|_| {
            error!("Invalid order status: {value}. But this conversion cannot fail. Defaulting to New");
            OrderStatusType::New
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
        write!(f, "#{}", self.0)
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
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: OrderStatusType,
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
    /// The total price of the order
    pub total_price: MicroTari,
    /// The currency of the order
    pub currency: String,
    /// The time the order was created on Shopify
    pub created_at: DateTime<Utc>,
}

impl NewOrder {
    pub fn new(order_id: OrderId, customer_id: String, total_price: MicroTari) -> Self {
        Self { order_id, customer_id, memo: None, total_price, currency: "XTR".to_string(), created_at: Utc::now() }
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

//--------------------------------------        OrderUpdate       ------------------------------------------------------

/// A struct representing the fields that are allowed to be updated on an order
#[derive(Debug, Clone, Default)]
pub struct OrderUpdate {
    /// An optional description supplied by the user for the order. Useful for matching orders with payments
    pub memo: Option<String>,
    /// The total price of the order
    pub total_price: Option<MicroTari>,
    /// The currency of the order
    pub currency: Option<String>,
    /// The new order status
    pub status: Option<OrderStatusType>,
}

impl OrderUpdate {
    pub fn is_empty(&self) -> bool {
        self.memo.is_none() && self.total_price.is_none() && self.currency.is_none() && self.status.is_none()
    }

    pub fn with_memo(mut self, memo: String) -> Self {
        self.memo = Some(memo);
        self
    }

    pub fn with_total_price(mut self, total_price: MicroTari) -> Self {
        self.total_price = Some(total_price);
        self
    }

    pub fn with_currency(mut self, currency: String) -> Self {
        self.currency = Some(currency);
        self
    }

    pub fn with_status(mut self, status: OrderStatusType) -> Self {
        self.status = Some(status);
        self
    }
}

//--------------------------------------        Payment       ---------------------------------------------------------
#[derive(Debug, Clone)]
pub struct Payment {
    pub txid: String,
    /// The time the payment was received
    pub created_at: DateTime<Utc>,
    /// The time the payment was updated
    pub updated_at: DateTime<Utc>,
    /// The public key of the user who made the payment
    pub sender: PublicKey,
    /// The amount of the payment
    pub amount: MicroTari,
    /// The memo attached to the transfer
    pub memo: Option<String>,
    pub payment_type: PaymentType,
    pub status: TransferStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentType {
    OnChain,
    Manual,
}

impl From<String> for PaymentType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "OnChain" => Self::OnChain,
            "Manual" => Self::Manual,
            _ => panic!("Invalid payment type: {}", value),
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

#[derive(Debug, Clone)]
pub struct NewPayment {
    /// The public key of the user who made the payment
    pub sender: TariAddress,
    /// The amount of the payment
    pub amount: MicroTari,
    /// The transaction identifier. Typically, the kernel signature in Tari
    pub txid: String,
    /// The memo attached to the transfer
    pub memo: Option<String>,
}

impl NewPayment {
    pub fn new(sender: TariAddress, amount: MicroTari, txid: String) -> Self {
        Self { sender, amount, txid, memo: None }
    }

    pub fn with_memo(mut self, memo: String) -> Self {
        self.memo = Some(memo);
        self
    }
}

//-----------------------------------------   PaymentStatus   ---------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    Received,
    Confirmed,
    Cancelled,
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
        match value.as_str() {
            "Received" => Self::Received,
            "Confirmed" => Self::Confirmed,
            "Cancelled" => Self::Cancelled,
            _ => panic!("Invalid transfer status: {}", value),
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
