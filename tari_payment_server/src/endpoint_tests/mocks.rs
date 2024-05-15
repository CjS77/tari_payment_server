use mockall::mock;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::{Order, OrderId, Role, UserAccount},
    AccountManagement,
    AuthApiError,
    AuthManagement,
};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub struct MockErr {
    pub message: String,
}

impl MockErr {
    pub fn new(message: &str) -> Self {
        Self { message: message.to_string() }
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
        async fn fetch_user_account_for_address(&self, address: &TariAddress) -> Result<Option<UserAccount>, MockErr>;
        async fn fetch_orders_for_account(&self, account_id: i64) -> Result<Vec<Order>, MockErr>;
    }
}

mock! {
    pub AuthManager {}
    impl AuthManagement for AuthManager {
        async fn update_nonce_for_address(&self, pubkey: &TariAddress, nonce: u64) -> Result<(), AuthApiError>;
        async fn create_auth_log(&self, pubkey: &TariAddress, nonce: u64) -> Result<(), AuthApiError>;
        async fn check_auth_account_exists(&self, address: &TariAddress) -> Result<bool, AuthApiError>;
        async fn check_address_has_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<(), AuthApiError>;
        async fn fetch_roles_for_address(&self, address: &TariAddress) -> Result<Vec<Role>, AuthApiError>;
        async fn assign_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<(), AuthApiError>;
        async fn remove_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<u64, AuthApiError>;
    }
}
