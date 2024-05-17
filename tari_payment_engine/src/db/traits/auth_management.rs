use tari_common_types::tari_address::TariAddress;

use crate::{db_types::Role, AuthApiError};

/// The `AuthManagement` trait defines behaviour for managing authentication and authorisation.
///
/// ## Authentication
/// For users to interact with the payment engine, they must be authenticated. This is done at the server level, but the
/// `AuthManagement` trait does provide some helper methods to help with the process.
///
/// Specifically, the [`create_auth_log`] and [`upsert_nonce_for_address`] methods are used to create and update login
/// records for users. See the Authentication documentation for [`tari_payment_server`] , which is stateless on the
/// user side. However, the server must keep track
/// of a nonce for each user to ensure that authentication tokens cannot be replayed.
#[allow(async_fn_in_trait)]
pub trait AuthManagement {
    /// Checks whether an account exists for the given address. The function succeeds if the query succeeds, returning
    /// the existence of the account as a boolean.
    async fn check_auth_account_exists(&self, address: &TariAddress) -> Result<bool, AuthApiError>;
    /// Checks whether an address is authorised for **all** of the given roles. The function only succeeds if this is
    /// the case. If any of the roles are missing, the error [`AuthApiError::RoleNotAllowed(usize)`] is returned,
    /// with the number of missing roles given as the parameter.
    /// You can use [`fetch_roles_for_address`] to get valid roles for the address.
    async fn check_address_has_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<(), AuthApiError>;
    /// Fetches the roles for the given address. If the address is not found, the request still succeeds and returns
    /// an empty vector.
    async fn fetch_roles_for_address(&self, address: &TariAddress) -> Result<Vec<Role>, AuthApiError>;

    /// Creates a new login record for the given address.
    async fn create_auth_log(&self, address: &TariAddress, nonce: u64) -> Result<(), AuthApiError>;

    /// Checks the nonce for the given address, creating a new login record if necessary. If the nonce is not strictly
    /// increasing, the error [`AuthApiError::InvalidNonce`] is returned.
    ///
    /// The default implementation of this function is to call [`check_auth_account_exists`] and [`create_auth_log`]
    async fn upsert_nonce_for_address(&self, address: &TariAddress, nonce: u64) -> Result<(), AuthApiError> {
        if self.check_auth_account_exists(address).await? {
            self.update_nonce_for_address(address, nonce).await
        } else {
            self.create_auth_log(address, nonce).await
        }
    }

    /// Updates the nonce for the given address. The nonce must be strictly increasing, otherwise the error
    /// [`AuthApiError::InvalidNonce`] is returned.
    async fn update_nonce_for_address(&self, address: &TariAddress, nonce: u64) -> Result<(), AuthApiError>;
    /// Assigns the given roles to the address. This function must be idempotent.
    async fn assign_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<(), AuthApiError>;

    /// Removes the given roles from the address. The number of roles actually removed is returned. This function must
    /// be idempotent.
    async fn remove_roles(&self, address: &TariAddress, roles: &[Role]) -> Result<u64, AuthApiError>;
}
