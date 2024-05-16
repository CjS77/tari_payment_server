use tari_common_types::tari_address::TariAddress;

use crate::{
    db_types::{NewOrder, NewPayment, Order, OrderId, OrderUpdate, Payment, Role, TransferStatus, UserAccount},
    AuthApiError,
};

pub enum InsertOrderResult {
    Inserted(i64),
    AlreadyExists(i64),
}

pub enum InsertPaymentResult {
    Inserted(String),
    AlreadyExists(String),
}

#[allow(async_fn_in_trait)]
pub trait PaymentGatewayDatabase: Clone {
    type Error: std::error::Error;

    /// The URL of the database
    fn url(&self) -> &str;

    /// Fetches the user account for the given order.
    ///
    /// If the account does not exist, one is created and the given customer id and/or public key is linked to the
    /// account.
    async fn fetch_or_create_account_for_order(&self, order: &NewOrder) -> Result<i64, Self::Error>;

    /// Fetches the user account for the given payment.
    ///
    /// If the account does not exist, one is created and the given public key and (if present) customer id is linked to
    /// the account.
    async fn fetch_or_create_account_for_payment(&self, payment: &Payment) -> Result<i64, Self::Error>;

    /// Takes a new order, and in a single atomic transaction,
    /// * calls `save_new_order` to store the order in the database. If the order already exists, nothing further is
    ///   done.
    /// * creates a new account for the customer if one does not already exist
    /// * Updates the total orders value for the account
    ///
    /// Returns the account id for the customer.
    async fn process_new_order_for_customer(&self, order: NewOrder) -> Result<i64, Self::Error>;

    /// Takes a new payment, and in a single atomic transaction,
    /// * calls `save_payment` to store the payment in the database. If the payment already exists, nothing further is
    ///   done.
    /// * The payment is marked as `Unconfirmed`
    /// * creates a new account for the public key if one does not already exist
    /// Returns the account id for the public key.
    async fn process_new_payment_for_pubkey(&self, payment: NewPayment) -> Result<i64, Self::Error>;

    /// Checks whether any orders associated with the given account id can be fulfilled.
    /// If no orders can be fulfilled, an empty vector is returned.
    async fn fetch_payable_orders(&self, account_id: i64) -> Result<Vec<Order>, Self::Error>;

    /// Tries to fulfil the list of arders given from the given account.
    ///
    /// Any order that has enough credit in the account
    /// * Will be marked as Paid
    /// * Returned in the result vector.
    async fn try_pay_orders(&self, account_id: i64, orders: &[Order]) -> Result<Vec<Order>, Self::Error>;

    /// Updates the payment status for the given transaction id. This is typically called to transition a payment from
    /// `Unconfirmed` to `Confirmed` or `Cancelled`.
    ///
    /// If the transaction was not "Received", an error is returned.
    /// If the status is unchanged, then nothing is changed, and `None` is returned.
    ///
    /// If the status is changed, the account id corresponding to the transaction is returned.
    async fn update_payment_status(&self, tx_id: &str, status: TransferStatus) -> Result<Option<i64>, Self::Error>;

    /// Updates the order details for the given order id. Not all fields are permitted to be updated, so
    /// `OrderUpdate` only exposes those that can be changed.
    async fn update_order(&self, id: &OrderId, update: OrderUpdate) -> Result<(), Self::Error>;

    /// Closes the database connection.
    async fn close(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[allow(async_fn_in_trait)]
pub trait OrderManagement {
    type Error: std::error::Error;

    async fn order_by_id(&self, order_id: &OrderId) -> Result<Option<Order>, Self::Error>;
}

#[allow(async_fn_in_trait)]
pub trait AccountManagement {
    type Error: std::error::Error;
    /// Fetches the user account associated with the given account id. If no account exists, `None` is returned.
    async fn fetch_user_account(&self, account_id: i64) -> Result<Option<UserAccount>, Self::Error>;

    /// Fetches the user account for the given order id. A user account must have already been created for this account.
    /// If no account is found, `None` will be returned.
    ///
    /// Alternatively, you can search through the memo fields of payments to find a matching order id by calling
    /// [`search_for_user_account_by_memo`].
    async fn fetch_user_account_for_order(&self, order_id: &OrderId) -> Result<Option<UserAccount>, Self::Error>;

    async fn fetch_user_account_for_customer_id(&self, customer_id: &str) -> Result<Option<UserAccount>, Self::Error>;
    async fn fetch_user_account_for_address(&self, address: &TariAddress) -> Result<Option<UserAccount>, Self::Error>;

    async fn fetch_orders_for_account(&self, account_id: i64) -> Result<Vec<Order>, Self::Error>;
}

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

#[macro_export]
macro_rules! op {
    (binary $for_struct:ident, $impl_trait:ident, $impl_fn:ident) => {
        impl $impl_trait for $for_struct {
            type Output = Self;

            fn $impl_fn(self, rhs: Self) -> Self::Output {
                Self(self.0.$impl_fn(rhs.0))
            }
        }
    };

    (inplace $for_struct:ident, $impl_trait:ident, $impl_fn:ident) => {
        impl $impl_trait for $for_struct {
            fn $impl_fn(&mut self, rhs: Self) {
                self.0.$impl_fn(rhs.0)
            }
        }
    };

    (unary $for_struct:ident, $impl_trait:ident, $impl_fn:ident) => {
        impl $impl_trait for $for_struct {
            type Output = Self;

            fn $impl_fn(self) -> Self::Output {
                Self(self.0.$impl_fn())
            }
        }
    };
}
