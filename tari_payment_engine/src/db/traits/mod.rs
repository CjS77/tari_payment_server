//! #  Database management and control.
//!
//! This module provides the interfaces that define the interface contracts of the payment engine database *backends*.
//!
//! ## Accounts
//! An account is a record that associates one or more Tari wallets (via their address) and their associated
//! payments with a set of orders from the merchant.
//!
//! The [`PaymentGatewayDatabase`] trait provides the mechanisms for matching Tari addresses with merchant accounts as
//! they enter the system. It is also responsible for updating account state (balances, order status etc.).
//!
//! The [`AccountManagement`] trait provides methods for querying information about these accounts. This also includes
//! queries for orders, payments and other account-related information.
//!
//! ## Traits
//! The [`r#mod`] module defines behavior that database backend need to expose in order to be supported by the
//! Tari Payment Engine.
//!
//! * [`PaymentGatewayDatabase`] defines the highest level of behavior for backends supporting the Tari Payment Engine.
//! * [`OrderManagement`] defines the behaviour for querying information about orders in the database backend.
//! * [`AuthManagement`] defines behavior for managing authentication.
//! * [`AccountManagement`] provides methods for querying information about user accounts, orders and payments.
mod account_management;
mod auth_management;
mod order_management;
mod payment_gateway_database;

mod data_objects;

pub use account_management::AccountManagement;
pub use auth_management::AuthManagement;
pub use data_objects::{InsertOrderResult, InsertPaymentResult};
pub use order_management::OrderManagement;
pub use payment_gateway_database::PaymentGatewayDatabase;

#[macro_export]
macro_rules! op {
    (binary $for_struct:ident, $impl_trait:ident, $impl_fn:ident) => {
        impl $impl_trait for $for_struct {
            type Output = Self;

            fn $impl_fn(self, rhs: Self) -> Self::Output {
                Self(self.0.$impl_fn(rhs.0))
            }
        }
    };

    (inplace $for_struct:ident, $impl_trait:ident, $impl_fn:ident) => {
        impl $impl_trait for $for_struct {
            fn $impl_fn(&mut self, rhs: Self) {
                self.0.$impl_fn(rhs.0)
            }
        }
    };

    (unary $for_struct:ident, $impl_trait:ident, $impl_fn:ident) => {
        impl $impl_trait for $for_struct {
            type Output = Self;

            fn $impl_fn(self) -> Self::Output {
                Self(self.0.$impl_fn())
            }
        }
    };
}
