use std::fmt::Debug;

use tari_common_types::tari_address::TariAddress;

use crate::{db::common::AuthManagement, db_types::Role, AuthApiError};

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
    pub async fn update_nonce_for_address(&self, address: &TariAddress, nonce: u64) -> Result<(), AuthApiError> {
        self.db.update_nonce_for_address(address, nonce).await
    }

    pub async fn check_address_has_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<(), AuthApiError> {
        self.db.check_address_has_roles(address, roles).await
    }
}
