use std::{fmt::Display, time::Duration};

use anyhow::Result;
use dialoguer::{console::Style, theme::ColorfulTheme, Confirm, FuzzySelect};
use indicatif::{ProgressBar, ProgressStyle};
use prettytable::{row, Cell, Row, Table};
use tari_payment_engine::db_types::UserAccount;
use tari_payment_server::data_objects::ExchangeRateResult;
use tpg_common::MicroTari;

use crate::{
    profile_manager::{read_config, Profile},
    tari_payment_server::client::PaymentServerClient,
};

pub const COMMANDS: [&str; 6] =
    ["Authenticate", "Exit", "My Account", "Server health", "Fetch Tari price", "Set Tari price"];

pub async fn run() -> Result<()> {
    let theme = ColorfulTheme { values_style: Style::new().yellow().dim(), ..ColorfulTheme::default() };
    let profile = select_profile(&theme)?;
    let mut client = PaymentServerClient::new(profile.clone());
    loop {
        let i = FuzzySelect::with_theme(&theme).with_prompt("Select command").items(&COMMANDS).interact()?;
        match COMMANDS[i] {
            "Authenticate" => handle_response(client.authenticate().await.map(|_| "Authenticated")),
            "Server health" => handle_response(client.health().await),
            "My Account" => handle_response(client.my_account().await.map(format_user_account)),
            "Fetch Tari price" => handle_response(client.fetch_exchange_rates("USD").await.map(format_exchange_rate)),
            "Set Tari price" => set_tari_price(&mut client).await?,
            "Exit" => break,
            _ => continue,
        }
    }
    Ok(())
}

fn handle_response<T: Display>(res: Result<T>) {
    match res {
        Ok(res) => println!("{res}"),
        Err(e) => println!("Error: {}", e),
    }
}

async fn set_tari_price(client: &mut PaymentServerClient) -> Result<()> {
    let rate = dialoguer::Input::<f64>::new().with_prompt("Enter Tari price (per USD)").interact()?;
    let price = MicroTari::from((rate * 1e6) as i64);
    if !Confirm::new().with_prompt(format!("Set Tari price to {price}?")).interact()? {
        return Err(anyhow::anyhow!("Cancelled"));
    }
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_style(
        ProgressStyle::with_template("{spinner:5} {msg} [{elapsed}]").unwrap()
            // For more spinners check out the cli-spinners project:
            // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
            .tick_strings(&[
                "🕛 ",
                "🕐 ",
                "🕑 ",
                "🕒 ",
                "🕓 ",
                "🕔 ",
                "🕕 ",
                "🕖 ",
                "🕗 ",
                "🕘 ",
                "🕙 ",
                "🕚 "
            ]),
    );
    pb.set_message("Updating prices (this could take a few minutes)...");
    let res = client.set_exchange_rate("USD", price).await;
    pb.finish_with_message("Done!");
    res
}

fn select_profile(theme: &ColorfulTheme) -> Result<Profile> {
    let user_data = read_config()?;
    let options = user_data.profiles.iter().map(|p| format!("{} ({})", p.name, p.server)).collect::<Vec<String>>();
    let profile =
        FuzzySelect::with_theme(theme).with_prompt("Select profile").items(&options).interact().and_then(|i| {
            let profile = &user_data.profiles[i];
            Ok(profile.clone())
        })?;
    Ok(profile)
}

fn format_user_account(account: UserAccount) -> String {
    let mut table = Table::new();
    table.add_row(row!["Field", "Value"]);
    table.add_row(Row::new(vec![Cell::new("ID"), Cell::new(&account.id.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Created At"), Cell::new(&account.created_at.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Updated At"), Cell::new(&account.updated_at.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Total Received"), Cell::new(&account.total_received.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Current Pending"), Cell::new(&account.current_pending.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Current Balance"), Cell::new(&account.current_balance.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Total Orders"), Cell::new(&account.total_orders.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Current Orders"), Cell::new(&account.current_orders.to_string())]));

    // Format the table to a string
    table.to_string()
}

fn format_exchange_rate(rate: ExchangeRateResult) -> String {
    let tari = MicroTari::from(rate.rate);
    format!("1 {} => {tari} (Last update: {})", rate.currency, rate.updated_at)
}
