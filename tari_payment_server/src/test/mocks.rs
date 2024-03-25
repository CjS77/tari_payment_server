use mockall::mock;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::db_types::{OrderId, UserAccount};
use tari_payment_engine::{AccountManagement, AuthApiError, AuthManagement};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub struct MockErr {
    pub message: String,
}

impl MockErr {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for MockErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

mock! {
    pub AccountManager {}
    impl AccountManagement for AccountManager {
        type Error = MockErr;
        async fn fetch_user_account(&self, account_id: i64) -> Result<Option<UserAccount>, MockErr>;
        async fn fetch_user_account_for_order(&self, order_id: &OrderId) -> Result<Option<UserAccount>, MockErr>;
        async fn search_for_user_account_by_memo(&self, memo_match: &str) -> Result<Option<i64>, MockErr>;
        async fn fetch_user_account_for_customer_id(&self, customer_id: &str) -> Result<Option<UserAccount>, MockErr>;
        async fn fetch_user_account_for_pubkey(&self, pubkey: &TariAddress) -> Result<Option<UserAccount>, MockErr>;
    }
}

mock! {
    pub AuthManager {}
    impl AuthManagement for AuthManager {
        async fn update_nonce_for_address(&self, pubkey: &TariAddress, nonce: u64) -> std::result::Result<Option<i64>, AuthApiError>;
    }
}
