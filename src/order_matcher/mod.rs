//! # SPG order matcher
//! This module hosts the order matcher for the SPG. It comprises three co-operating components:
//!
//! # OrderWatcher
//! [`OrderWatcher`] subscribes to [`NewOrder`] messages from the [`server`].
//! When a new order arrives, it will create, or update a new order record in the database.
//! It will then send a [`OrderCreated`] or [`OrderUpdated`] message to the OrderMatcher.
//! It also sends an [`OrderReceivedEvent`] message to all subscribers.
//!
//! # PaymentWatcher
//! [`PaymentWatcher`] subscribes to [`PaymentReceived`] messages from a [`PaymentReceiver`].
//! The payment receiver is typically a hot wallet, but it needn't be.
//! When a payment is received, the payment watcher will create a payment record in the database. This is an idempotent
//! operation, so if the payment has already been received, it will have no net effect.
//! Payment watch then sends a [`PaymentUpdate`] message to the OrderMatcher.
//! Additional payments for the same order will result in additional [`PaymentUpdate`] messages. Payment watcher does
//! not know anything about the order, so it cannot determine if the payment is for the correct amount. It just
//! forwards payment information to the order matcher.
//!
//! # Order matcher
//! [`OrderMatcher`] responds to order and payment messages from the [`OrderWatcher`] and [`PaymentWatcher`]
//! respectively. On receiving new information, it will attempt to match orders with payments.
//! When a payment and order is matched an order status record is created in the database.
//! An order status has a state of:
//! * `Pending`: The order has been matched with a payment, but the payment has not yet been confirmed.
//! * `Confirmed`: The payment has been confirmed. The order is now considered to be paid and can be fulfilled.
//! * `PartiallyPaid`: The order has been matched with a payment, but the payment is less than the order total.
//! * `Overpaid`: The order has been matched with a payment, but the payment is greater than the order total. The
//!    overpaid amount should be refunded.
//! * `Cancelled`: The order has been cancelled. The matching payment should be refunded.
//! * `Expired`: The order has expired. No payment was ever received. Items can be placed back in stock.
//!
//! Order matcher also broadcasts a [`OrderStatus`] message to all subscribers.
//!
//! [`OrderMatcher`] will also respond to the following admin messages:
//! * [`OverrideOrderResolution`]: This message can be used to override the order resolution state.
//! * [`AdjustPayment`]: This message can be used to adjust the payment amount for a given order. Amount can be
//! positive or negative, for example if a refund has been given.

pub mod messages;

pub mod order_watcher;
pub mod payment_watcher;
