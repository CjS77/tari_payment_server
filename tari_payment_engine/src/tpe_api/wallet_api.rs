use std::{fmt::Debug, net::IpAddr};

use log::trace;
use serde::Serialize;
use tari_common_types::tari_address::TariAddress;

use crate::{
    helpers::WalletSignature,
    traits::{NewWalletInfo, WalletAuth, WalletAuthApiError, WalletInfo, WalletManagement, WalletManagementError},
};

#[derive(Clone)]
pub struct WalletAuthApi<B> {
    db: B,
}

impl<B: Debug> Debug for WalletAuthApi<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WalletApi ({:?})", self.db)
    }
}

impl<B> WalletAuthApi<B> {
    pub fn new(db: B) -> Self {
        Self { db }
    }
}

impl<B> WalletAuthApi<B>
where B: WalletAuth
{
    pub async fn get_wallet_info(&self, address: &TariAddress) -> Result<WalletInfo, WalletAuthApiError> {
        let info = self.db.get_wallet_info(address).await?;
        Ok(info)
    }

    pub async fn update_wallet_nonce(&self, address: &TariAddress, new_nonce: i64) -> Result<(), WalletAuthApiError> {
        self.db.update_wallet_nonce(address, new_nonce).await?;
        Ok(())
    }

    /// Authenticates the wallet signature against the state stored in the database.
    ///
    /// In particular:
    /// - The signature is internally valid
    /// - The address of the wallet sending the message matches the record in the database
    /// - The nonce is greater than the nonce stored in the database
    /// - The remote IP address matches the IP address stored in the database
    /// - Updating the nonce in the database is successful
    pub async fn authenticate_wallet<T: Serialize>(
        &self,
        sig: WalletSignature,
        remote_ip: &IpAddr,
        payload: &T,
    ) -> Result<(), WalletAuthApiError> {
        if !sig.is_valid(payload) {
            return Err(WalletAuthApiError::InvalidSignature);
        }
        trace!("Wallet signature for {} is valid", sig.address.as_base58());
        let address = sig.address.as_address();
        let wallet_info = self.db.get_wallet_info(address).await?;
        if wallet_info.address != sig.address {
            return Err(WalletAuthApiError::WalletNotFound);
        }
        // The DB will usually trigger a constraint violation if the nonce is not greater than the last nonce,
        // but we check here in case the backend does not
        if wallet_info.last_nonce >= sig.nonce {
            return Err(WalletAuthApiError::InvalidNonce);
        }
        if wallet_info.ip_address != *remote_ip {
            return Err(WalletAuthApiError::InvalidIpAddress);
        }
        self.update_wallet_nonce(address, sig.nonce).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct WalletManagementApi<B> {
    db: B,
}

impl<B> WalletManagementApi<B> {
    pub fn new(db: B) -> Self {
        Self { db }
    }
}

impl<B> WalletManagementApi<B>
where B: WalletManagement
{
    pub async fn fetch_authorized_wallets(&self) -> Result<Vec<WalletInfo>, WalletManagementError> {
        self.db.fetch_authorized_wallets().await
    }

    pub async fn register_wallet(&self, new_wallet_info: NewWalletInfo) -> Result<(), WalletManagementError> {
        self.db.register_wallet(new_wallet_info).await
    }

    pub async fn deregister_wallet(&self, address: &TariAddress) -> Result<(), WalletManagementError> {
        self.db.deregister_wallet(address).await
    }
}
