use crate::db::common::AuthManagement;
use crate::AuthApiError;
use std::fmt::Debug;
use tari_common_types::tari_address::TariAddress;

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
where
    B: AuthManagement,
{
    pub async fn update_nonce_for_address(
        &self,
        pubkey: &TariAddress,
        nonce: u64,
    ) -> Result<Option<i64>, AuthApiError> {
        self.db.update_nonce_for_address(pubkey, nonce).await
    }
}
