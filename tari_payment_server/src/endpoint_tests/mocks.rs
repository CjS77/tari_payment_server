use mockall::mock;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::{Order, OrderId, Payment, Role, UserAccount},
    order_objects::OrderQueryFilter,
    tpe_api::account_objects::{FullAccount, Pagination},
    traits::{AccountApiError, AccountManagement, AuthApiError, AuthManagement},
};

mock! {
    pub AccountManager {}
    impl AccountManagement for AccountManager {
        async fn fetch_user_account(&self, account_id: i64) -> Result<Option<UserAccount>, AccountApiError>;
        async fn fetch_user_account_for_order(&self, order_id: &OrderId) -> Result<Option<UserAccount>, AccountApiError>;
        async fn fetch_user_account_for_customer_id(&self, customer_id: &str) -> Result<Option<UserAccount>, AccountApiError>;
        async fn fetch_user_account_for_address(&self, address: &TariAddress) -> Result<Option<UserAccount>, AccountApiError>;
        async fn fetch_orders_for_account(&self, account_id: i64) -> Result<Vec<Order>, AccountApiError>;
        async fn fetch_order_by_order_id(&self, order_id: &OrderId) -> Result<Option<Order>, AccountApiError>;
        async fn fetch_payments_for_address(&self, address: &TariAddress) -> Result<Vec<Payment>, AccountApiError>;
        async fn history_for_address(&self, address: &TariAddress) -> Result<Option<FullAccount>, AccountApiError>;
        async fn history_for_id(&self, account_id: i64) -> Result<Option<FullAccount>, AccountApiError>;
        async fn search_orders(&self, query: OrderQueryFilter, only_address: Option<TariAddress>) -> Result<Vec<Order>, AccountApiError>;
        async fn creditors(&self) -> Result<Vec<UserAccount>, AccountApiError>;
        async fn fetch_customer_ids(&self, pagination: &Pagination) -> Result<Vec<String>, AccountApiError>;
        async fn fetch_addresses(&self, pagination: &Pagination) -> Result<Vec<TariAddress>, AccountApiError>;
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
