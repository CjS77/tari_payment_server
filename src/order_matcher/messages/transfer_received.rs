use crate::db::models::{MicroTari, PublicKey};
use actix::Message;
use chrono::{DateTime, Utc};

/// A message containing information about a Tari transfer that has been picked up by the listener
#[derive(Clone, Debug, Message)]
#[rtype(result = "()")]
pub struct TransferReceived {
    /// The time the payment was received
    pub timestamp: DateTime<Utc>,
    /// The block height of the transaction
    pub block_height: u64,
    /// The public key of the user who made the payment
    pub sender: PublicKey,
    /// The public key of the user who received the payment
    pub receiver: PublicKey,
    /// The amount of the payment
    pub amount: MicroTari,
    /// The memo attached to the transfer
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TransferBuilder {
    timestamp: Option<DateTime<Utc>>,
    block_height: Option<u64>,
    sender: Option<PublicKey>,
    receiver: Option<PublicKey>,
    amount: Option<MicroTari>,
    memo: Option<String>,
}

impl TransferBuilder {
    pub fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn block_height(mut self, block_height: u64) -> Self {
        self.block_height = Some(block_height);
        self
    }

    pub fn sender(mut self, sender: PublicKey) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn receiver(mut self, receiver: PublicKey) -> Self {
        self.receiver = Some(receiver);
        self
    }

    pub fn amount(mut self, amount: MicroTari) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn memo<S: Into<String>>(mut self, memo: S) -> Self {
        self.memo = Some(memo.into());
        self
    }

    pub fn build(self) -> TransferReceived {
        TransferReceived {
            timestamp: self.timestamp.unwrap_or_else(Utc::now),
            block_height: self.block_height.unwrap_or_default(),
            sender: self.sender.unwrap_or_else(|| {
                "0000000000000000000000000000000000000000000000000000000000000001".into()
            }),
            receiver: self.receiver.unwrap_or_else(|| {
                "0000000000000000000000000000000000000000000000000000000000000002".into()
            }),
            amount: self.amount.unwrap_or_else(|| MicroTari::from(1000)),
            memo: self.memo,
        }
    }
}
