//! Unifies API for accessing accounts.

use std::fmt::Debug;

use tari_common_types::tari_address::TariAddress;

use crate::{
    db_types::{Order, UserAccount},
    order_manager::errors::AccountApiError,
    AccountManagement,
};

pub struct AccountApi<B> {
    db: B,
}

impl<B: Debug> Debug for AccountApi<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AccountApi ({:?})", self.db)
    }
}

impl<B> AccountApi<B>
where B: AccountManagement
{
    pub fn new(db: B) -> Self {
        Self { db }
    }

    /// Fetches the user account for the given account id. If no account exists, `None` is returned.
    pub async fn account_by_id(&self, account_id: i64) -> Result<Option<UserAccount>, AccountApiError> {
        self.db.fetch_user_account(account_id).await.map_err(|e| AccountApiError::DatabaseError(e.to_string()))
    }

    /// Fetches the user account for the given Tari address.
    pub async fn account_by_address(&self, address: &TariAddress) -> Result<Option<UserAccount>, AccountApiError> {
        self.db.fetch_user_account_for_address(address).await.map_err(|e| AccountApiError::DatabaseError(e.to_string()))
    }

    pub async fn orders_for_address(&self, address: &TariAddress) -> Result<Option<Vec<Order>>, AccountApiError> {
        let id = match self.account_by_address(address).await {
            Ok(Some(acc)) => acc.id,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        };
        self.db
            .fetch_orders_for_account(id)
            .await
            .map(|v| Some(v))
            .map_err(|e| AccountApiError::DatabaseError(e.to_string()))
    }
}
