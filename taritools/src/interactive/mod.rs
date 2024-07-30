use std::{
    fmt::{Display, Write},
    net::IpAddr,
    time::Duration,
};

use anyhow::Result;
use dialoguer::{console::Style, theme::ColorfulTheme, Confirm, FuzzySelect, Select};
use indicatif::{ProgressBar, ProgressStyle};
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::{OrderId, SerializedTariAddress},
    traits::NewWalletInfo,
};
use tari_payment_server::data_objects::{ModifyOrderParams, MoveOrderParams, UpdateMemoParams};
use tokio::join;
use tpg_common::MicroTari;

use crate::{
    interactive::{
        formatting::{
            format_addresses_with_qr_code,
            format_exchange_rate,
            format_full_account,
            format_order,
            format_order_result,
            format_orders,
            format_payments_result,
            format_user_account,
            format_wallet_list,
            print_order,
        },
        menus::{top_menu, Menu},
        selector::{AddressSelector, CustomerSelector},
    },
    profile_manager::{read_config, Profile},
    tari_payment_server::client::PaymentServerClient,
};

pub mod formatting;
pub mod menus;

pub mod selector;

pub struct InteractiveApp {
    client: Option<PaymentServerClient>,
    current_menu: &'static Menu,
    breadcrumbs: Vec<&'static Menu>,
    customer_ids: CustomerSelector,
    addresses: AddressSelector,
}

impl InteractiveApp {
    pub fn new() -> Self {
        let client = None;
        let current_menu = top_menu();
        let breadcrumbs = vec![top_menu()];
        let customer_ids = CustomerSelector::default();
        let addresses = AddressSelector::default();
        Self { client, current_menu, breadcrumbs, customer_ids, addresses }
    }

    pub fn is_logged_in(&self) -> bool {
        self.client.is_some()
    }

    pub async fn login(&mut self) -> Result<String> {
        if self.is_logged_in() {
            return Ok("Logged In".to_string());
        }
        let theme = ColorfulTheme { values_style: Style::new().yellow().dim(), ..ColorfulTheme::default() };
        let profile = select_profile(&theme)?;
        let mut client = PaymentServerClient::new(profile);
        client.authenticate().await?;
        self.client = Some(client);
        Ok("Logged In".to_string())
    }

    pub fn menu_prompt(&self) -> String {
        let breadcrumbs = self.breadcrumbs.iter().map(|m| m.0).collect::<Vec<&str>>().join(" » ");
        let status = if self.is_logged_in() {
            let client = self.client.as_ref().unwrap();
            format!("{client}")
        } else {
            String::from("Not logged in")
        };
        format!("{breadcrumbs:-30}{status:50}")
    }

    pub fn pop_menu(&mut self) {
        if self.breadcrumbs.len() > 1 {
            self.breadcrumbs.pop();
            self.current_menu = self.breadcrumbs.last().unwrap();
        }
    }

    pub fn select_menu(&mut self, menu: &'static Menu) {
        self.breadcrumbs.push(menu);
        self.current_menu = menu;
    }

    pub async fn run(&mut self) -> Result<()> {
        print_logo();
        loop {
            let theme = ColorfulTheme { prompt_style: Style::new().magenta().bold(), ..ColorfulTheme::default() };
            let i = FuzzySelect::with_theme(&theme)
                .with_prompt(self.menu_prompt())
                .items(self.current_menu.1)
                .interact()?;
            match self.current_menu.1[i] {
                "Server health" => self.server_health().await,
                "My Account" => self.my_account().await,
                "My Orders" => self.my_orders().await,
                "My Open Orders" => self.my_unfulfilled_orders().await,
                "My Payments" => self.my_payments().await,
                "Account History" => handle_response(self.my_history().await),
                "Admin Menu" => self.select_menu(menus::admin_menu()),
                "User Menu" => self.select_menu(menus::user_menu()),
                "Cancel Order" => handle_response(self.cancel_order().await),
                "Reset Order" => handle_response(self.reset_order().await),
                "Mark order as Paid" => handle_response(self.fulfil_order().await),
                "Fetch Tari price" => self.fetch_tari_price().await,
                "Set Tari price" => self.set_tari_price().await,
                "Issue Credit" => handle_response(self.issue_credit().await),
                "Order by Id" => handle_response(self.order_by_id().await),
                "Orders for Address" => handle_response(self.orders_for_address().await),
                "Payments for Address" => handle_response(self.payments_for_address().await),
                "History for Address" => handle_response(self.history_for_address().await),
                "History for Account Id" => handle_response(self.history_for_id().await),
                "Edit memo" => handle_response(self.edit_memo().await),
                "Reassign Order" => handle_response(self.reassign_order().await),
                "List payment addresses" => handle_response(self.get_payment_addresses().await),
                "Add authorized wallet" => handle_response(self.add_authorized_wallet().await),
                "Remove authorized wallets" => handle_response(self.remove_authorized_wallet().await),
                "List authorized wallets" => handle_response(self.list_authorized_wallets().await),
                "Logout" => self.logout(),
                "Back" => self.pop_menu(),
                "Exit" => break,
                _ => continue,
            }
        }
        Ok(())
    }

    fn logout(&mut self) {
        self.client = None;
        println!("Logged out");
    }

    async fn server_health(&self) {
        let client = PaymentServerClient::new(Profile::default());
        handle_response(client.health().await)
    }

    async fn get_payment_addresses(&mut self) -> Result<String> {
        let client = PaymentServerClient::new(Profile::default());
        let addresses = client.payment_addresses().await?;
        Ok(format_addresses_with_qr_code(&addresses))
    }

    async fn list_authorized_wallets(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let client = self.client.as_ref().unwrap();
        let wallets = client.authorized_wallets().await?;
        Ok(format_wallet_list(&wallets))
    }

    async fn add_authorized_wallet(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let address =
            dialoguer::Input::<String>::new().with_prompt("Tari address for new payment wallet:").interact()?;
        let address = SerializedTariAddress::from(TariAddress::from_hex(&address)?);
        let ip_address =
            dialoguer::Input::<String>::new().with_prompt("IP address for new payment wallet:").interact()?;
        let ip_address = ip_address.parse::<IpAddr>()?;
        let new_wallet = NewWalletInfo { address, ip_address, initial_nonce: None };
        let client = self.client.as_ref().unwrap();
        client.add_authorized_wallet(&new_wallet).await?;
        Ok("New wallet has been added successfully".into())
    }

    async fn remove_authorized_wallet(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let client = self.client.as_ref().unwrap();
        let addresses = client.payment_addresses().await?;
        let items =
            addresses.iter().map(|a| format!("{} ({})", a.to_hex(), a.to_emoji_string())).collect::<Vec<String>>();
        let idx = Select::new().with_prompt("Select wallet to remove").items(&items).interact()?;
        let address = &addresses[idx];
        client.remove_authorized_wallet(address).await?;
        Ok(format!("Wallet {address} has been removed successfully"))
    }

    async fn my_account(&mut self) {
        let mut res = self.login().await;
        if res.is_ok() {
            res = self.client.as_mut().unwrap().my_account().await.map(format_user_account);
        }
        handle_response(res)
    }

    async fn order_by_id(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let order_id = dialoguer::Input::<String>::new().with_prompt("Enter order ID").interact()?;
        let order = self
            .client
            .as_mut()
            .unwrap()
            .order_by_id(&OrderId::new(order_id))
            .await?
            .ok_or(anyhow::anyhow!("Order does not exist"))?;
        print_order(&order)
    }

    async fn my_orders(&mut self) {
        let mut res = self.login().await;
        if res.is_ok() {
            res = self.client.as_mut().unwrap().my_orders().await.and_then(format_order_result);
        }
        handle_response(res)
    }

    async fn my_unfulfilled_orders(&mut self) {
        let mut res = self.login().await;
        if res.is_ok() {
            res = self.client.as_mut().unwrap().my_unfulfilled_orders().await.map(|o| format_orders(&o));
        }
        handle_response(res)
    }

    async fn my_payments(&mut self) {
        let mut res = self.login().await;
        if res.is_ok() {
            res = self.client.as_mut().unwrap().my_payments().await.and_then(format_payments_result);
        }
        handle_response(res)
    }

    async fn my_history(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let client = self.client.as_ref().unwrap();
        let history = client.my_history().await?;
        format_full_account(history)
    }

    async fn history_for_address(&mut self) -> Result<String> {
        let _unused = self.login().await;
        let address = self.select_address().await?;
        let client = self.client.as_ref().unwrap();
        let history = client.history_for_address(&address).await?;
        format_full_account(history)
    }

    async fn history_for_id(&mut self) -> Result<String> {
        let _unused = self.login().await;
        let client = self.client.as_ref().unwrap();
        let account_id = dialoguer::Input::<i64>::new().with_prompt("Enter account id (NOT customer id)").interact()?;
        let history = client.history_for_id(account_id).await?;
        format_full_account(history)
    }

    async fn fetch_tari_price(&mut self) {
        let mut res = self.login().await;
        if res.is_ok() {
            res = self.client.as_mut().unwrap().fetch_exchange_rates("USD").await.map(format_exchange_rate);
        }
        handle_response(res)
    }

    async fn set_tari_price(&mut self) {
        let mut res = self.login().await;
        if res.is_ok() {
            res = set_new_tari_price(self.client.as_mut().unwrap()).await
        }
        handle_response(res)
    }

    fn get_modify_order_params(&self) -> Result<ModifyOrderParams> {
        let order_id = dialoguer::Input::<String>::new().with_prompt("Enter order ID").interact()?;
        let order_id = OrderId::new(order_id);
        let reason = dialoguer::Input::<String>::new().with_prompt("Enter reason").interact()?;
        Ok(ModifyOrderParams { order_id, reason })
    }

    async fn cancel_order(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let params = self.get_modify_order_params()?;
        let client = self.client.as_ref().unwrap();
        let order = client.cancel_order(&params).await?;
        print_order(&order)
    }

    async fn fulfil_order(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let params = self.get_modify_order_params()?;
        let client = self.client.as_ref().unwrap();
        let order = client.fulfil_order(&params).await?;
        print_order(&order)
    }

    async fn reset_order(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let order_id = dialoguer::Input::<String>::new().with_prompt("Enter order ID").interact()?;
        let order_id = OrderId::new(order_id);
        let client = self.client.as_ref().unwrap();
        let order = client.reset_order(&order_id).await?;
        print_order(&order)
    }

    async fn issue_credit(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let client = self.client.as_ref().unwrap();
        self.customer_ids.update(client).await?;
        let idx = FuzzySelect::new().with_prompt("Select customer ID").items(self.customer_ids.items()).interact()?;
        let cust_id = &self.customer_ids.items()[idx];
        let amount = input_tari_amount("Enter amount in Tari:")?;
        let reason = dialoguer::Input::<String>::new().with_prompt("Enter reason").interact()?;
        let orders = client.issue_credit(cust_id, amount, reason).await?;
        if orders.is_empty() {
            Ok("Credit issued successfully".into())
        } else {
            println!("Credit issued successfully.\nThe following {} orders have been paid as a result:", orders.len());
            Ok(format_orders(&orders))
        }
    }

    async fn orders_for_address(&mut self) -> Result<String> {
        let _unused = self.login().await;
        let address = self.select_address().await?;
        let client = self.client.as_ref().unwrap();
        client.orders_for_address(address).await.and_then(format_order_result)
    }

    async fn payments_for_address(&mut self) -> Result<String> {
        let _unused = self.login().await;
        let address = self.select_address().await?;
        let client = self.client.as_ref().unwrap();
        let payments = client.payments_for_address(address).await?;
        format_payments_result(payments)
    }

    async fn select_address(&mut self) -> Result<TariAddress> {
        let _unused = self.login().await;
        let client = self.client.as_ref().unwrap();
        self.addresses.update(client).await?;
        let idx = FuzzySelect::new().with_prompt("Select address").items(self.addresses.items()).interact()?;
        let address = TariAddress::from_hex(&self.addresses.items()[idx])?;
        Ok(address)
    }

    async fn edit_memo(&mut self) -> Result<String> {
        let _unused = self.login().await;
        let params = self.get_modify_order_params()?;
        let client = self.client.as_ref().unwrap();
        let order =
            client.order_by_id(&params.order_id).await?.ok_or_else(|| anyhow::anyhow!("Order does not exist"))?;
        let old_memo = order.memo.unwrap_or_else(|| "Provide a new memo".to_string());
        let new_memo = dialoguer::Editor::new().edit(&old_memo)?.ok_or_else(|| anyhow::anyhow!("No memo provided"))?;
        let params = UpdateMemoParams { order_id: params.order_id, new_memo, reason: Some(params.reason) };
        let order = client.edit_memo(&params).await?;
        print_order(&order)
    }

    async fn reassign_order(&mut self) -> Result<String> {
        let _unused = self.login().await;
        let params = self.get_modify_order_params()?;
        let client = self.client.as_ref().unwrap();
        let (cust_ids_res, order_res) = join!(self.customer_ids.update(client), client.order_by_id(&params.order_id));
        cust_ids_res?;
        let order = order_res?.ok_or_else(|| anyhow::anyhow!("Order {} does not exist", params.order_id))?;
        let formatted_order = print_order(&order)?;
        println!("You're about to re-assign this order:\n{formatted_order}");
        let idx =
            FuzzySelect::new().with_prompt("Select new customer ID").items(self.customer_ids.items()).interact()?;
        let new_customer_id = self.customer_ids.items()[idx].to_string();
        let params = MoveOrderParams { order_id: params.order_id, new_customer_id, reason: params.reason };
        let result = client.reassign_order(&params).await?;
        let mut msg = format!("# Order {} reassignment summary\n", params.order_id);
        writeln!(
            msg,
            "**Customer id** changed from {} to {}",
            result.orders.old_order.customer_id, result.orders.new_order.customer_id
        )?;
        writeln!(msg, "*Account id** changed from {} to {}", result.old_account_id, result.new_account_id)?;
        if result.is_filled {
            writeln!(msg, "The new account had sufficient credit to cover the order and it has been marked as PAID")?;
        }
        writeln!(msg, "\n## Old order")?;
        format_order(&result.orders.old_order, &mut msg)?;
        writeln!(msg, "## New order")?;
        format_order(&result.orders.new_order, &mut msg)?;
        Ok(msg)
    }
}

fn print_logo() {
    const LOGO: &str = include_str!("../../../assets/logo.txt");
    println!("{LOGO}");
}

fn handle_response<T: Display>(res: Result<T>) {
    match res {
        Ok(res) => println!("{res}"),
        Err(e) => println!("Error: {}", e),
    }
}

fn input_tari_amount(prompt: &str) -> Result<MicroTari> {
    let rate = dialoguer::Input::<f64>::new().with_prompt(prompt).interact()?;
    #[allow(clippy::cast_possible_truncation)]
    let price = MicroTari::from((rate * 1e6) as i64);
    if Confirm::new().with_prompt(format!("Confirm value of {price}?")).interact()? {
        Ok(price)
    } else {
        Err(anyhow::anyhow!("Cancelled"))
    }
}

async fn set_new_tari_price(client: &mut PaymentServerClient) -> Result<String> {
    let price = input_tari_amount("Enter Tari price (per USD)")?;
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_style(
        ProgressStyle::with_template("{spinner:5} {msg} [{elapsed}]")
            .unwrap()
            .tick_strings(&["🕛 ", "🕐 ", "🕑 ", "🕒 ", "🕓 ", "🕔 ", "🕕 ", "🕖 ", "🕗 ", "🕘 ", "🕙 ", "🕚 "]),
    );
    pb.set_message("Updating prices (this could take a few minutes)...");
    match client.set_exchange_rate("USD", price).await {
        Ok(()) => {
            pb.finish_with_message("Done!");
            Ok("Tari price set successfully".into())
        },
        Err(e) => {
            pb.finish_with_message("Error!");
            Err(e)
        },
    }
}

fn select_profile(theme: &ColorfulTheme) -> Result<Profile> {
    let user_data = read_config()?;
    let options = user_data.profiles.iter().map(|p| format!("{} ({})", p.name, p.server)).collect::<Vec<String>>();
    let profile = FuzzySelect::with_theme(theme).with_prompt("Select profile").items(&options).interact().map(|i| {
        let profile = &user_data.profiles[i];
        profile.clone()
    })?;
    Ok(profile)
}
