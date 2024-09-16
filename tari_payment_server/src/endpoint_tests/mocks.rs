use mockall::mock;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::{AddressBalance, CustomerBalance, CustomerOrderBalance, CustomerOrders, Order, OrderId, Payment, Role},
    order_objects::OrderQueryFilter,
    tpe_api::account_objects::{AddressHistory, CustomerHistory, Pagination},
    traits::{AccountApiError, AccountManagement, AuthApiError, AuthManagement},
};

mock! {
    pub AccountManager {}
    impl AccountManagement for AccountManager {
        async fn fetch_order_by_order_id(&self, order_id: &OrderId) -> Result<Option<Order>, AccountApiError>;
        async fn fetch_payments_for_address(&self, address: &TariAddress) -> Result<Vec<Payment>, AccountApiError>;
        async fn history_for_address(&self, address: &TariAddress) -> Result<AddressHistory, AccountApiError>;
        async fn search_orders(&self, query: OrderQueryFilter) -> Result<Vec<Order>, AccountApiError>;
        async fn creditors(&self) -> Result<Vec<CustomerOrders>, AccountApiError>;
        async fn fetch_customer_ids(&self, pagination: &Pagination) -> Result<Vec<String>, AccountApiError>;
        async fn fetch_addresses(&self, pagination: &Pagination) -> Result<Vec<TariAddress>, AccountApiError>;
        async fn fetch_orders_for_address(&self, address: &TariAddress) -> Result<Vec<Order>, AccountApiError>;
        async fn fetch_address_balance(&self, address: &TariAddress) -> Result<AddressBalance, AccountApiError>;
        async fn fetch_customer_balance(&self, customer_id: &str) -> Result<CustomerBalance, AccountApiError>;
        async fn history_for_customer(&self, customer_id: &str) -> Result<CustomerHistory, AccountApiError>;
        async fn fetch_customer_order_balance(&self, customer_id: &str) -> Result<CustomerOrderBalance, AccountApiError>;
        async fn fetch_customer_ids_for_address(&self, address: &TariAddress) -> Result<Vec<String>, AccountApiError>;
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
