use std::fmt::Debug;

use tari_common_types::tari_address::TariAddress;

use crate::{
    db_types::Role,
    traits::{AuthApiError, AuthManagement},
};

pub struct AuthApi<B> {
    db: B,
}

impl<B: Debug> Debug for AuthApi<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AuthApi ({:?})", self.db)
    }
}

impl<B> AuthApi<B> {
    pub fn new(db: B) -> Self {
        Self { db }
    }
}

impl<B> AuthApi<B>
where B: AuthManagement
{
    pub async fn upsert_nonce_for_address(&self, address: &TariAddress, nonce: u64) -> Result<(), AuthApiError> {
        self.db.upsert_nonce_for_address(address, nonce).await
    }

    pub async fn check_address_has_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<(), AuthApiError> {
        self.db.check_address_has_roles(address, roles).await
    }

    pub async fn check_auth_account_exists(&self, address: &TariAddress) -> Result<bool, AuthApiError> {
        self.db.check_auth_account_exists(address).await
    }

    pub async fn fetch_roles_for_address(&self, address: &TariAddress) -> Result<Vec<Role>, AuthApiError> {
        self.db.fetch_roles_for_address(address).await
    }

    pub async fn assign_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<(), AuthApiError> {
        self.db.assign_roles(address, roles).await
    }

    pub async fn remove_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<u64, AuthApiError> {
        self.db.remove_roles(address, roles).await
    }
}
