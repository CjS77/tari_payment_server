use std::{
    fmt::{Display, Write},
    net::IpAddr,
    time::Duration,
};

use anyhow::Result;
use dialoguer::{console::Style, theme::ColorfulTheme, Confirm, FuzzySelect, MultiSelect, Select};
use indicatif::{ProgressBar, ProgressStyle};
use menus::commands::*;
use tari_common::configuration::Network;
use tari_common_types::tari_address::{TariAddress, TariAddressFeatures};
use tari_crypto::{
    keys::PublicKey,
    ristretto::{RistrettoPublicKey, RistrettoSecretKey},
    tari_utilities::hex::Hex,
};
use tari_payment_engine::{
    db_types::{OrderId, Role, SerializedTariAddress},
    helpers::MemoSignature,
    traits::NewWalletInfo,
};
use tari_payment_server::data_objects::{ModifyOrderParams, MoveOrderParams, UpdateMemoParams};
use tokio::join;
use tpg_common::MicroTari;
use zeroize::Zeroize;

use crate::{
    interactive::{
        formatting::{
            format_addresses_with_qr_code,
            format_claimed_order,
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
        seed_phrase::{seed_words_to_comms_key, string_to_seed_words},
        selector::{AddressSelector, CustomerSelector},
    },
    profile_manager::{read_config, write_config, Profile},
    tari_payment_server::client::PaymentServerClient,
};

pub mod formatting;
pub mod menus;

pub mod seed_phrase;
pub mod selector;

struct ProfileInfo {
    client: PaymentServerClient,
    profile: Profile,
}

pub struct InteractiveApp {
    user: Option<ProfileInfo>,
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
        Self { user: client, current_menu, breadcrumbs, customer_ids, addresses }
    }

    pub fn is_logged_in(&self) -> bool {
        self.user.is_some()
    }

    pub async fn login(&mut self) -> Result<String> {
        if self.is_logged_in() {
            return Ok("Logged In".to_string());
        }
        let theme = ColorfulTheme { values_style: Style::new().yellow().dim(), ..ColorfulTheme::default() };
        let profile = select_profile(&theme)?;
        let mut client = PaymentServerClient::new(profile.clone());
        client.authenticate().await?;
        let info = ProfileInfo { client, profile };
        self.user = Some(info);
        Ok("Logged In".to_string())
    }

    pub fn menu_prompt(&self) -> String {
        let breadcrumbs = self.breadcrumbs.iter().map(|m| m.0).collect::<Vec<&str>>().join(" » ");
        let status = if self.is_logged_in() {
            let client = self.user.as_ref().expect("User is logged in. Client should not be None");
            client.profile.name.as_str()
        } else {
            "Not logged in"
        };
        format!("{breadcrumbs:-30}{status:50}")
    }

    pub fn pop_menu(&mut self) {
        if self.breadcrumbs.len() > 1 {
            self.breadcrumbs.pop();
            self.current_menu = self.breadcrumbs.last().unwrap_or(&top_menu());
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
                SERVER_HEALTH => self.server_health().await,
                MY_ACCOUNT => self.my_account().await,
                MY_ORDERS => self.my_orders().await,
                CLAIM_ORDER => handle_response(self.claim_order().await),
                MY_OPEN_ORDERS => self.my_unfulfilled_orders().await,
                MY_PAYMENTS => self.my_payments().await,
                MY_ACCOUNT_HISTORY => handle_response(self.my_history().await),
                NAV_TO_ADMIN_MENU => self.select_menu(menus::admin_menu()),
                NAV_TO_USER_MENU => self.select_menu(menus::user_menu()),
                CANCEL => handle_response(self.cancel_order().await),
                RESET_ORDER => handle_response(self.reset_order().await),
                MARK_ORDER_PAID => handle_response(self.fulfil_order().await),
                FETCH_PRICE => self.fetch_tari_price().await,
                SET_PRICE => self.set_tari_price().await,
                ISSUE_CREDIT => handle_response(self.issue_credit().await),
                ORDER_BY_ID => handle_response(self.order_by_id().await),
                ORDERS_FOR_ADDRESS => handle_response(self.orders_for_address().await),
                PAYMENTS_FOR_ADDRESS => handle_response(self.payments_for_address().await),
                HISTORY_FOR_ADDRESS => handle_response(self.history_for_address().await),
                HISTORY_FOR_ACCOUNT_ID => handle_response(self.history_for_id().await),
                EDIT_MEMO => handle_response(self.edit_memo().await),
                REASSIGN_ORDER => handle_response(self.reassign_order().await),
                LIST_PAYMENT_ADDRESSES => handle_response(self.get_payment_addresses().await),
                ADD_AUTH_WALLET => handle_response(self.add_authorized_wallet().await),
                REMOVE_AUTH_WALLETS => handle_response(self.remove_authorized_wallet().await),
                LIST_AUTH_WALLETS => handle_response(self.list_authorized_wallets().await),
                ADD_PROFILE => handle_response(self.add_profile().await),
                LOGOUT => self.logout(),
                NAV_BACK => self.pop_menu(),
                EXIT => break,
                _ => continue,
            }
        }
        Ok(())
    }

    fn logout(&mut self) {
        self.user = None;
        println!("Logged out");
    }

    fn client_mut(&mut self) -> Option<&mut PaymentServerClient> {
        self.user.as_mut().map(|u| &mut u.client)
    }

    fn client(&self) -> Option<&PaymentServerClient> {
        self.user.as_ref().map(|u| &u.client)
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
        let client = self.client().expect("User is logged in. Client should not be None");
        let wallets = client.authorized_wallets().await?;
        Ok(format_wallet_list(&wallets))
    }

    async fn add_authorized_wallet(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let address =
            dialoguer::Input::<String>::new().with_prompt("Tari address for new payment wallet:").interact()?;
        let address = SerializedTariAddress::from(TariAddress::from_base58(&address)?);
        let ip_address =
            dialoguer::Input::<String>::new().with_prompt("IP address for new payment wallet:").interact()?;
        let ip_address = ip_address.parse::<IpAddr>()?;
        let new_wallet = NewWalletInfo { address, ip_address, initial_nonce: None };
        let client = self.client().expect("User is logged in. Client should not be None");
        client.add_authorized_wallet(&new_wallet).await?;
        Ok("New wallet has been added successfully".into())
    }

    async fn remove_authorized_wallet(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let client = self.client().expect("User is logged in. Client should not be None");
        let addresses = client.payment_addresses().await?;
        let items =
            addresses.iter().map(|a| format!("{} ({})", a.to_base58(), a.to_emoji_string())).collect::<Vec<String>>();
        let idx = Select::new().with_prompt("Select wallet to remove").items(&items).interact()?;
        let address = &addresses[idx];
        client.remove_authorized_wallet(address).await?;
        Ok(format!("Wallet {address} has been removed successfully"))
    }

    async fn my_account(&mut self) {
        let mut res = self.login().await;
        if res.is_ok() {
            res = self
                .client()
                .expect("User is logged in. Client should not be None")
                .my_account()
                .await
                .map(format_user_account);
        }
        handle_response(res)
    }

    async fn order_by_id(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let order_id = dialoguer::Input::<String>::new().with_prompt("Enter order ID").interact()?;
        let order = self
            .client()
            .expect("User is logged in. Client should not be None")
            .order_by_id(&OrderId::new(order_id))
            .await?
            .ok_or(anyhow::anyhow!("Order does not exist"))?;
        print_order(&order)
    }

    async fn my_orders(&mut self) {
        let mut res = self.login().await;
        if res.is_ok() {
            res = self
                .client()
                .expect("User is logged in. Client should not be None")
                .my_orders()
                .await
                .and_then(format_order_result);
        }
        handle_response(res)
    }

    async fn my_unfulfilled_orders(&mut self) {
        let mut res = self.login().await;
        if res.is_ok() {
            res = self
                .client()
                .expect("User is logged in. Client should not be None")
                .my_unfulfilled_orders()
                .await
                .map(|o| format_orders(&o));
        }
        handle_response(res)
    }

    async fn my_payments(&mut self) {
        let mut res = self.login().await;
        if res.is_ok() {
            res = self
                .client()
                .expect("User is logged in. Client should not be None")
                .my_payments()
                .await
                .and_then(format_payments_result);
        }
        handle_response(res)
    }

    async fn my_history(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let client = self.client().expect("User is logged in. Client should not be None");
        let history = client.my_history().await?;
        format_full_account(history)
    }

    async fn history_for_address(&mut self) -> Result<String> {
        let _unused = self.login().await;
        let address = self.select_address().await?;
        let client = self.client().expect("User is logged in. Client should not be None");
        let history = client.history_for_address(&address).await?;
        format_full_account(history)
    }

    async fn history_for_id(&mut self) -> Result<String> {
        let _unused = self.login().await;
        let client = self.client().expect("User is logged in. Client should not be None");
        let account_id = dialoguer::Input::<i64>::new().with_prompt("Enter account id (NOT customer id)").interact()?;
        let history = client.history_for_id(account_id).await?;
        format_full_account(history)
    }

    async fn fetch_tari_price(&mut self) {
        let mut res = self.login().await;
        if res.is_ok() {
            res = self
                .client()
                .expect("User is logged in. Client should not be None")
                .fetch_exchange_rates("USD")
                .await
                .map(format_exchange_rate);
        }
        handle_response(res)
    }

    async fn set_tari_price(&mut self) {
        let mut res = self.login().await;
        if res.is_ok() {
            res = set_new_tari_price(self.client_mut().expect("User is logged in. Client should not be None")).await
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
        let client = self.client().expect("User is logged in. Client should not be None");
        let order = client.cancel_order(&params).await?;
        print_order(&order)
    }

    async fn fulfil_order(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let params = self.get_modify_order_params()?;
        let client = self.client().expect("User is logged in. Client should not be None");
        let order = client.fulfil_order(&params).await?;
        print_order(&order)
    }

    async fn reset_order(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let order_id = dialoguer::Input::<String>::new().with_prompt("Enter order ID").interact()?;
        let order_id = OrderId::new(order_id);
        let client = self.client().expect("User is logged in. Client should not be None");
        let order = client.reset_order(&order_id).await?;
        print_order(&order)
    }

    async fn issue_credit(&mut self) -> Result<String> {
        let _unused = self.login().await?;
        let client = &self.user.as_ref().expect("User is logged in. Client should not be None").client;
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
        let client = self.client().expect("User is logged in. Client should not be None");
        client.orders_for_address(address).await.and_then(format_order_result)
    }

    async fn payments_for_address(&mut self) -> Result<String> {
        let _unused = self.login().await;
        let address = self.select_address().await?;
        let client = self.client().expect("User is logged in. Client should not be None");
        let payments = client.payments_for_address(address).await?;
        format_payments_result(payments)
    }

    async fn select_address(&mut self) -> Result<TariAddress> {
        let _s = self.login().await?;
        let client = &self.user.as_ref().expect("User is logged in. Client should not be None").client;
        self.addresses.update(client).await?;
        let idx = FuzzySelect::new().with_prompt("Select address").items(self.addresses.items()).interact()?;
        let address = TariAddress::from_base58(&self.addresses.items()[idx])?;
        Ok(address)
    }

    async fn edit_memo(&mut self) -> Result<String> {
        let _unused = self.login().await;
        let params = self.get_modify_order_params()?;
        let client = self.client().expect("User is logged in. Client should not be None");
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
        let client = &self.user.as_ref().expect("User is logged in. Client should not be None").client;
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

    async fn add_profile(&mut self) -> Result<String> {
        let name = dialoguer::Input::<String>::new().with_prompt("Enter profile name").interact()?;
        let secret_type = Select::new()
            .with_prompt("Select secret type")
            .items(&["Provide secret key", "Provide seed phrase", "Use environment variable"])
            .interact()?;
        let (address, secret_key, secret_key_envar) = match secret_type {
            0 => {
                let secret_key =
                    dialoguer::Input::<String>::new().with_prompt("Enter secret key (in hex):").interact()?;
                let key = RistrettoSecretKey::from_hex(&secret_key)?;
                let address = confirm_address(&key)?;
                let secret_key = Some(key);
                (address, secret_key, None)
            },
            1 => {
                let seed_phrase =
                    dialoguer::Input::<String>::new().with_prompt("Enter seed phrase (space separated):").interact()?;
                let seed_words = string_to_seed_words(seed_phrase)?;
                let key = seed_words_to_comms_key(seed_words)?;
                let address = confirm_address(&key)?;
                let secret_key = Some(key);
                (address, secret_key, None)
            },
            2 => {
                let envar =
                    dialoguer::Input::<String>::new().with_prompt("Enter environment variable name").interact()?;
                let key = std::env::var(&envar)?;
                let mut key = RistrettoSecretKey::from_hex(&key)?;
                let address = confirm_address(&key)?;
                key.zeroize();
                let secret_key_envar = Some(envar);
                (address, None, secret_key_envar)
            },
            _ => unreachable!(),
        };
        let roles = MultiSelect::new()
            .with_prompt("Select roles")
            .items(&["User", "ReadAll", "Write", "SuperAdmin"])
            .interact()?;
        let roles = roles
            .iter()
            .map(|i| match i {
                0 => Role::User,
                1 => Role::ReadAll,
                2 => Role::Write,
                3 => Role::SuperAdmin,
                _ => unreachable!(),
            })
            .collect();
        let server = dialoguer::Input::<String>::new().with_prompt("Enter server URL").interact()?;
        let server = url::Url::parse(&server)?;
        let mut user_data = read_config()?;
        let profile = Profile { name, address, secret_key, secret_key_envar, roles, server };
        user_data.profiles.push(profile);
        write_config(&user_data)?;
        Ok("Profile added successfully".into())
    }

    async fn claim_order(&mut self) -> Result<String> {
        let _unused = self.login().await;
        let order_id = dialoguer::Input::<String>::new().with_prompt("Enter order ID").interact()?;
        let order_id = OrderId::new(order_id);
        let ProfileInfo { client, profile } =
            self.user.as_ref().expect("User is logged in. Profile should not be None");
        let key = profile.secret_key().ok_or(anyhow::anyhow!("No secret key found for profile"))?;
        let address = profile.address.as_address().clone();
        let signature = MemoSignature::create(address, order_id.to_string(), &key)?;
        let order = client.claim_order(&signature).await?;
        format_claimed_order(&order)
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

fn confirm_address(secret_key: &RistrettoSecretKey) -> Result<SerializedTariAddress> {
    let network = Select::new().with_prompt("Select network").items(&["Mainnet", "Stagenet", "Nextnet"]).interact()?;
    let network = match network {
        0 => Network::MainNet,
        1 => Network::StageNet,
        2 => Network::NextNet,
        _ => unreachable!(),
    };
    let pubkey = RistrettoPublicKey::from_secret_key(secret_key);
    let address = TariAddress::new_single_address(pubkey, network, TariAddressFeatures::default());
    let confirm = Confirm::new()
        .with_prompt(format!("Use address {} ({})?", address.to_base58(), address.to_emoji_string()))
        .interact()?;
    if confirm {
        Ok(SerializedTariAddress::from(address))
    } else {
        Err(anyhow::anyhow!("Cancelled"))
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
            .expect("Hardcoded progress template is invalid. Report this to the developers")
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
