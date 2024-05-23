use std::{fmt::Debug, net::SocketAddr};

use tari_common_types::tari_address::TariAddress;

use crate::{
    helpers::WalletSignature,
    traits::{WalletAuth, WalletAuthApiError, WalletInfo},
};

pub struct WalletApi<B> {
    db: B,
}

impl<B: Debug> Debug for WalletApi<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WalletApi ({:?})", self.db)
    }
}

impl<B> WalletApi<B> {
    pub fn new(db: B) -> Self {
        Self { db }
    }
}

impl<B> WalletApi<B>
where B: WalletAuth
{
    pub async fn get_wallet_info(&self, address: &TariAddress) -> Result<WalletInfo, WalletAuthApiError> {
        let info = self.db.get_wallet_info(address).await?;
        Ok(info)
    }

    pub async fn update_wallet_nonce(&self, address: &TariAddress, new_nonce: i64) -> Result<(), WalletAuthApiError> {
        let _ = self.db.update_wallet_nonce(address, new_nonce).await?;
        Ok(())
    }

    /// Authenticates the wallet signature against the state stored in the database.
    ///
    /// In particular:
    /// - The signature is internally valid
    /// - The nonce is greater than the nonce stored in the database
    /// - The remote IP address matches the IP address stored in the database
    /// - Updating the nonce in the database is successful
    pub async fn authenticate_wallet(
        &self,
        sig: WalletSignature,
        remote_ip: SocketAddr,
    ) -> Result<(), WalletAuthApiError> {
        if !sig.is_valid() {
            return Err(WalletAuthApiError::InvalidSignature);
        }
        let address = sig.address.as_address();
        let wallet_info = self.db.get_wallet_info(address).await?;
        if wallet_info.last_nonce <= sig.nonce {
            return Err(WalletAuthApiError::InvalidNonce);
        }
        if wallet_info.ip_address != remote_ip {
            return Err(WalletAuthApiError::InvalidIpAddress);
        }
        self.update_wallet_nonce(address, sig.nonce).await?;
        Ok(())
    }
}
