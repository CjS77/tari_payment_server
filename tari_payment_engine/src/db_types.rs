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
    helpers::{extract_and_verify_memo_signature, MemoSignatureError},
    tpe_api::order_objects::{address_to_base58, str_to_address},
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
    #[serde(serialize_with = "address_to_base58", deserialize_with = "str_to_address")] TariAddress,
);

impl SerializedTariAddress {
    pub fn to_address(self) -> TariAddress {
        self.0
    }

    pub fn as_address(&self) -> &TariAddress {
        &self.0
    }

    #[deprecated(since = "0.3.0", note = "Use as_base58 instead")]
    pub fn as_hex(&self) -> String {
        self.0.to_hex()
    }

    pub fn as_base58(&self) -> String {
        self.0.to_base58()
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
        write!(f, "{}", self.as_base58())
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
        self.as_base58().hash(state);
    }
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
    pub alt_id: Option<OrderId>,
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

//--------------------------------------        NewOrder       ---------------------------------------------------------
#[derive(Debug, Clone)]
pub struct NewOrder {
    /// The order_id as assigned by Shopify
    pub order_id: OrderId,
    /// An alternative order_id that can be used to reference the order
    pub alt_order_id: Option<OrderId>,
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
            alt_order_id: None,
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
            self.alt_order_id == order.alt_id &&
            self.customer_id == order.customer_id &&
            self.memo == order.memo &&
            self.total_price == order.total_price &&
            self.currency == order.currency &&
            self.created_at == order.created_at
    }
}

impl Display for NewOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let alt_str = match &self.alt_order_id {
            Some(alt) => format!(" ({alt})"),
            None => "".to_string(),
        };
        write!(
            f,
            "Order {order_id}{alt_str} @ \"{customer_id}\". {total_price} ({created_at})",
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AddressBalance {
    address: SerializedTariAddress,
    /// the sum of all Tari wallet transfers that have been confirmed
    total_confirmed: MicroTari,
    /// the total value of all orders that have been fulfilled
    total_paid: MicroTari,
    /// the current balance of the address (total_confirmed - total_paid)
    current_balance: MicroTari,
    last_update: DateTime<Utc>,
}

impl AddressBalance {
    pub fn new(address: TariAddress) -> Self {
        Self {
            address: SerializedTariAddress::from(address),
            total_confirmed: MicroTari::from_tari(0),
            total_paid: MicroTari::from_tari(0),
            current_balance: MicroTari::from_tari(0),
            last_update: Utc::now(),
        }
    }

    pub fn address(&self) -> &TariAddress {
        self.address.as_address()
    }

    pub fn total_confirmed(&self) -> MicroTari {
        self.total_confirmed
    }

    pub fn total_paid(&self) -> MicroTari {
        self.total_paid
    }

    pub fn current_balance(&self) -> MicroTari {
        self.current_balance
    }

    pub fn last_update(&self) -> DateTime<Utc> {
        self.last_update
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerBalance {
    total_confirmed: MicroTari,
    total_paid: MicroTari,
    current_balance: MicroTari,
    addresses: Vec<AddressBalance>,
}

impl CustomerBalance {
    pub fn new(balances: Vec<AddressBalance>) -> Self {
        let total_confirmed = balances.iter().map(|b| b.total_confirmed).sum();
        let total_paid = balances.iter().map(|b| b.total_paid).sum();
        let current_balance = balances.iter().map(|b| b.current_balance).sum();
        Self { total_confirmed, total_paid, current_balance, addresses: balances }
    }

    pub fn total_confirmed(&self) -> MicroTari {
        self.total_confirmed
    }

    pub fn total_paid(&self) -> MicroTari {
        self.total_paid
    }

    pub fn current_balance(&self) -> MicroTari {
        self.current_balance
    }

    pub fn addresses(&self) -> &[AddressBalance] {
        &self.addresses
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSettlementJournalEntry {
    pub order_id: OrderId,
    pub payment_address: SerializedTariAddress,
    pub settlement_type: SettlementType,
    pub amount: MicroTari,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SettlementJournalEntry {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub order_id: OrderId,
    pub payment_address: SerializedTariAddress,
    pub settlement_type: SettlementType,
    pub amount: MicroTari,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SettlementType {
    // Indicates that the order was paid from multiple addresses
    Multiple,
    // Indicates that the order was paid from a single address
    Single,
}

impl Display for SettlementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettlementType::Multiple => write!(f, "Multiple"),
            SettlementType::Single => write!(f, "Single"),
        }
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CustomerOrders {
    pub customer_id: String,
    pub status: OrderStatusType,
    pub total_orders: MicroTari,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustomerOrderBalance {
    pub customer_id: String,
    pub total_current: MicroTari,
    pub total_paid: MicroTari,
    pub total_expired: MicroTari,
    pub total_cancelled: MicroTari,
}

impl CustomerOrderBalance {
    pub fn new(balances: &[CustomerOrders]) -> Self {
        if balances.is_empty() {
            return Self::default();
        }
        let customer_id = balances[0].customer_id.clone();
        use OrderStatusType::*;
        let total_current =
            balances.iter().filter(|b| [New, Unclaimed].contains(&b.status)).map(|b| b.total_orders).sum();
        let total_paid = balances.iter().filter(|b| b.status == Paid).map(|b| b.total_orders).sum();
        let total_expired = balances.iter().filter(|b| b.status == Expired).map(|b| b.total_orders).sum();
        let total_cancelled = balances.iter().filter(|b| b.status == Cancelled).map(|b| b.total_orders).sum();
        Self { customer_id, total_current, total_paid, total_expired, total_cancelled }
    }
}
