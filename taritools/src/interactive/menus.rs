//! Interactive menu options.
//!
//! Commands must be unique across all menus. If the same name is used in multiple menus, the same function will be
//! called for each menu that contains the command.
pub type Menu = (&'static str, &'static [&'static str]);


pub mod commands {
    pub const CANCEL: &str = "Cancel Order";
    pub const MARK_PAID: &str = "Mark order as Paid";
    pub const RESET: &str = "Reset Order";
    pub const FETCH_PRICE: &str = "Fetch Tari price";
    pub const SET_PRICE: &str = "Set Tari price";
    pub const LOGOUT: &str = "Logout";
    pub const BACK: &str = "Back";
    pub const ISSUE_CREDIT: &str = "Issue Credit";
    pub const ORDER_BY_ID: &str = "Order by Id";
    pub const ORDERS_FOR_ADDRESS: &str = "Orders for Address";
    pub const PAYMENTS_FOR_ADDRESS: &str = "Payments for Address";
    pub const HISTORY_FOR_ADDRESS: &str = "History for Address";
    pub const HISTORY_FOR_ACCOUNT_ID: &str = "History for Account Id";
    pub const EDIT_MEMO: &str = "Edit memo";
    pub const REASSIGN_ORDER: &str = "Reassign Order";
    pub const ADD_AUTH_WALLET: &str = "Add authorized wallet";
    pub const REMOVE_AUTH_WALLETS: &str = "Remove authorized wallets";
    pub const LIST_AUTH_WALLETS: &str = "List authorized wallets";
    pub const SERVER_HEALTH: &str = "Server health";
    pub const EXIT: &str = "Exit";
    pub const ADD_PROFILE: &str = "Add profile";
    pub const CLAIM_ORDER: &str = "Claim Order";
    pub const MY_ACCOUNT: &str = "My Account";
    pub const MY_ORDERS: &str = "My Orders";
    pub const MY_OPEN_ORDERS: &str = "My Open Orders";
    pub const MY_PAYMENTS: &str = "My Payments";
    pub const ACCOUNT_HISTORY: &str = "Account History";
    pub const LIST_PAYMENT_ADDRESSES: &str = "List payment addresses";

    pub const ADMIN_MENU_STR: &str = "Admin Menu";
    pub const USER_MENU_STR: &str = "User Menu";
    pub const LOGOUT_STR: &str = "Logout";
    pub const EXIT_STR: &str = "Exit";
}

pub use commands::*;

pub const TOP_MENU: [&str; 4] = [
    ADMIN_MENU_STR,
    USER_MENU_STR,
    LOGOUT_STR,
    EXIT_STR,
];

pub const ADMIN_MENU: [&str; 20] = [
    CANCEL,
    MARK_PAID,
    RESET,
    FETCH_PRICE,
    SET_PRICE,
    LOGOUT,
    BACK,
    ISSUE_CREDIT,
    ORDER_BY_ID,
    ORDERS_FOR_ADDRESS,
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
    MY_ACCOUNT,
    LOGOUT,
    BACK,
    EXIT,
    MY_ORDERS,
    MY_OPEN_ORDERS,
    MY_PAYMENTS,
    ACCOUNT_HISTORY,
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
