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

pub mod auth;
pub mod cli;
pub mod config;
pub mod errors;

pub mod helpers;
pub mod routes;
pub mod server;

pub mod shopify_order;

#[cfg(test)]
mod endpoint_tests;
