//! # SPG server
//! This module hosts the server code for the SPG. It is responsible for:
//! Listening for incoming webhook requests from Shopify.
//! Parsing the request body and extracting the order information.
//! Sending the order information to all subscribers, including and primarily the order matcher.
//!
//! ## Configuration
//! The server is configured via environment variables. See [config](config/index.html) for more information.
//!
//! ## Routes
//! The server exposes the following routes:
//! * `/health`: A health check route that returns a 200 OK response.
//! * `/webhook/checkout_create`: The webhook route for receiving checkout create events from Shopify.

#![feature(type_alias_impl_trait)]

/// # Authentication
///
/// Tari Payment Server uses a JWT access token model for authentication. Users 'log in' or
/// authenticate with a login token signed with their Tari wallet private key.
///
/// Users must supply a login token in the `tpg_auth_token` header.
/// The token contains the following fields (See [`tari_payment_engine::db_types::LoginToken`]):
/// * `address` - The address of the user's wallet. This is the same as the pubkey with an additional checksum/network
///   byte.
/// * `nonce` - A unique number that must increase on every call (not necessarily by 1 - a unix time epoch can be used,
///   for example).
/// * `desired_roles` - A list of roles that the user wants to have. This MUST be a subset of the set of Roles that the
///   wallet address is authorised for (SuperAdmin can manipulate this set using the `/roles` endpoint).
///
/// The server uses JWT middleware to validate the token signature and the Tari Payment Engine
/// [`tari_payment_engine::traits::AuthManagement`] API to check that a valid nonce has been provided and that the roles
/// requested are a subset of the permitted [`tari_payment_engine::db_types::Role`]s.
///
/// If successful, the server returns an access token. The JWT is valid for a relatively short period and will NOT
/// refresh. It's designed for the most common use-case flow of:
///  * Authenticate (login)
///  * Perform an order or payment operation
///  * Go away (and let the token expire)

/// The server validates the login token, and provides an access token is response.
///
/// To access authenticated endpoints, the user must provide the access token in the `tpg_access_token` header with
/// every request.
///
/// The [`middleware::AclMiddlewareService`] middleware checks that
/// * the signature on the access token is valid
/// * the token hasn't expired
/// * The user is authorised for the Roles specified on the endpoint.
pub mod auth;
pub mod cli;
pub mod config;
pub mod data_objects;
pub mod errors;

pub mod expiry_worker;

pub mod helpers;

pub mod middleware;

pub mod routes;
pub mod server;

pub mod integrations;

#[cfg(test)]
mod endpoint_tests;
