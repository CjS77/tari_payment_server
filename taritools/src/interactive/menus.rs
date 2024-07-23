//! Interactive menu options.
//!
//! Commands must be unique across all menus. If the same name is used in multiple menus, the same function will be
//! called for each menu that contains the command.
pub type Menu = (&'static str, &'static [&'static str]);

pub const TOP_MENU: [&str; 5] = ["Admin Menu", "User Menu", "Server health", "Logout", "Exit"];
pub const ADMIN_MENU: [&str; 5] = ["Fetch Tari price", "Set Tari price", "Logout", "Back", "Exit"];
pub const USER_MENU: [&str; 7] = ["My Account", "Logout", "Back", "Exit", "My Orders", "My Open Orders", "My Payments"];

pub fn top_menu() -> &'static Menu {
    &("Main", &TOP_MENU)
}

pub fn admin_menu() -> &'static Menu {
    &("Admin", &ADMIN_MENU)
}

pub fn user_menu() -> &'static Menu {
    &("User", &USER_MENU)
}
