use crate::db::models::MicroTari;
use actix::Message;

#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct PaymentUpdate {
    /// The public key of the user who made the payment
    user_id: PublicKey,
    /// The amount of the payment
    amount: MicroTari,
    /// The order id the payment is for
    memo: Option<String>,
}

#[derive(Clone, Debug)]
pub enum PaymentSource {
    Wallet,
    ManualAdjustment,
}

/// A lightweight wrapper around a string representing a public key
#[derive(Clone, Debug)]
pub struct PublicKey {
    pub key: String,
}
