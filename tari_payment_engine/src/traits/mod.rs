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
//! The module defines behavior that database backend need to expose in order to be supported by the
//! Tari Payment Engine.
//!
//! * [`PaymentGatewayDatabase`] defines the highest level of behavior for backends supporting the Tari Payment Engine.
//! * [`AuthManagement`] defines behavior for managing authentication.
//! * [`AccountManagement`] provides methods for querying information about user accounts, orders and payments.
//! * [`WalletManagement`] defines behavior for managing the set of authorized hot wallets associated with the server.
mod account_management;
mod auth_management;

mod exchange_rates;
mod payment_gateway_database;

mod wallet_management;

mod data_objects;

pub use account_management::{AccountApiError, AccountManagement};
pub use auth_management::{AuthApiError, AuthManagement};
pub use data_objects::{ExpiryResult, MultiAccountPayment, NewWalletInfo, OrderMovedResult, WalletInfo};
pub use exchange_rates::{ExchangeRateError, ExchangeRates};
pub use payment_gateway_database::{PaymentGatewayDatabase, PaymentGatewayError};
pub use wallet_management::{WalletAuth, WalletAuthApiError, WalletManagement, WalletManagementError};
