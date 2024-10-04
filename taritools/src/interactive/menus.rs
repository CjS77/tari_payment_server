//! Interactive menu options.
//!
//! Commands must be unique across all menus. If the same name is used in multiple menus, the same function will be
//! called for each menu that contains the command.
pub type Menu = (&'static str, &'static [&'static str]);

// Command aliases. Keep this list in alphabetical order.
pub mod commands {
    pub const ADD_AUTH_WALLET: &str = "Add authorized wallet";
    pub const ADD_PROFILE: &str = "Add profile";
    pub const BALANCE_FOR_ADDRESS: &str = "Balance for Address";
    pub const CANCEL: &str = "Cancel Order";
    pub const CLAIM_ORDER: &str = "Claim Order";
    pub const CREDITORS: &str = "Get all unpaid orders";
    pub const EDIT_MEMO: &str = "Edit memo";
    pub const EXIT: &str = "Exit";
    pub const FETCH_PAYMENTS_FOR_ORDER: &str = "Fetch Payments for Order";
    pub const FETCH_PRICE: &str = "Fetch Tari price";
    pub const HISTORY_FOR_ACCOUNT_ID: &str = "History for Customer Id";
    pub const HISTORY_FOR_ADDRESS: &str = "History for Address";
    pub const ISSUE_CREDIT: &str = "Issue Credit";
    pub const LIST_AUTH_WALLETS: &str = "List authorized wallets";
    pub const LIST_PAYMENT_ADDRESSES: &str = "List payment addresses";
    pub const LOGOUT: &str = "Logout";
    pub const MARK_ORDER_PAID: &str = "Mark order as Paid";
    pub const MY_ACCOUNT_HISTORY: &str = "Account History";
    pub const MY_BALANCE: &str = "My Balance";
    pub const MY_OPEN_ORDERS: &str = "My Open Orders";
    pub const MY_ORDERS: &str = "My Orders";
    pub const MY_PAYMENTS: &str = "My Payments";
    pub const NAV_BACK: &str = "Back";
    pub const NAV_TO_ADMIN_MENU: &str = "Admin Menu";
    pub const NAV_TO_USER_MENU: &str = "User Menu";
    pub const ORDER_BY_ID: &str = "Order by Id";
    pub const ORDERS_FOR_ADDRESS: &str = "Orders for Address";
    pub const PAYMENTS_FOR_ADDRESS: &str = "Payments for Address";
    pub const REASSIGN_ORDER: &str = "Reassign Order";
    pub const REMOVE_AUTH_WALLETS: &str = "Remove authorized wallets";
    pub const RESET_ORDER: &str = "Reset Order";
    pub const SERVER_HEALTH: &str = "Server health";
    pub const SET_PRICE: &str = "Set Tari price";
}

pub use commands::*;

pub const TOP_MENU: [&str; 4] = [NAV_TO_ADMIN_MENU, NAV_TO_USER_MENU, LOGOUT, EXIT];

pub const ADMIN_MENU: [&str; 23] = [
    CANCEL,
    MARK_ORDER_PAID,
    RESET_ORDER,
    FETCH_PRICE,
    SET_PRICE,
    LOGOUT,
    NAV_BACK,
    ISSUE_CREDIT,
    CREDITORS,
    ORDER_BY_ID,
    BALANCE_FOR_ADDRESS,
    ORDERS_FOR_ADDRESS,
    FETCH_PAYMENTS_FOR_ORDER,
    PAYMENTS_FOR_ADDRESS,
    HISTORY_FOR_ADDRESS,
    HISTORY_FOR_ACCOUNT_ID,
    EDIT_MEMO,
    REASSIGN_ORDER,
    ADD_AUTH_WALLET,
    REMOVE_AUTH_WALLETS,
    LIST_AUTH_WALLETS,
    SERVER_HEALTH,
    EXIT,
];

pub const USER_MENU: [&str; 11] = [
    ADD_PROFILE,
    CLAIM_ORDER,
    LOGOUT,
    NAV_BACK,
    EXIT,
    MY_BALANCE,
    MY_ORDERS,
    MY_OPEN_ORDERS,
    MY_PAYMENTS,
    MY_ACCOUNT_HISTORY,
    LIST_PAYMENT_ADDRESSES,
];

pub fn top_menu() -> &'static Menu {
    &("Main", &TOP_MENU)
}

pub fn admin_menu() -> &'static Menu {
    &("Admin", &ADMIN_MENU)
}

pub fn user_menu() -> &'static Menu {
    &("User", &USER_MENU)
}
