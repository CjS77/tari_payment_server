use tari_common_types::tari_address::TariAddress;
use thiserror::Error;

use crate::traits::data_objects::{NewWalletInfo, UpdateWalletInfo, WalletInfo};

#[allow(async_fn_in_trait)]
pub trait WalletAuth {
    /// Retrieves the whitelisted IP address and nonce for the given wallet address
    async fn get_wallet_info(&self, wallet_address: &TariAddress) -> Result<WalletInfo, WalletAuthApiError>;
    async fn update_wallet_nonce(&self, wallet_address: &TariAddress, new_nonce: i64)
        -> Result<(), WalletAuthApiError>;
}

#[allow(async_fn_in_trait)]
pub trait WalletManagement {
    type Error: std::error::Error;

    /// Adds the given wallet info to the wallet auth table in the database.
    async fn register_wallet(&self, wallet: NewWalletInfo) -> Result<(), Self::Error>;

    /// Removes the wallet with the given address from the wallet auth table in the database.
    async fn deregister_wallet(&self, wallet_address: &TariAddress) -> Result<WalletInfo, Self::Error>;

    /// Updates the wallet with the given address in the wallet auth table in the database.
    async fn update_wallet_info(&self, wallet: UpdateWalletInfo) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Error)]
pub enum WalletAuthApiError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Nonce is not strictly increasing.")]
    InvalidNonce,
    #[error("The given address is not an authorized wallet")]
    WalletNotFound,
    #[error("The wallet authorization signature is invalid")]
    InvalidSignature,
    #[error("The wallet authorization IP address is invalid")]
    InvalidIpAddress,
}

impl From<sqlx::Error> for WalletAuthApiError {
    fn from(e: sqlx::Error) -> Self {
        WalletAuthApiError::DatabaseError(e.to_string())
    }
}
