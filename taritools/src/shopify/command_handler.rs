use shopify_tools::{ExchangeRate, ShopifyApi, ShopifyConfig};

use crate::shopify::{command_def::RatesCommand, OrdersCommand, ShopifyCommand};

pub async fn handle_shopify_command(command: ShopifyCommand) {
    use ShopifyCommand::*;
    match command {
        Orders(orders_command) => match orders_command {
            OrdersCommand::Get { id } => fetch_shopify_order(id).await,
            OrdersCommand::Cancel { id } => cancel_shopify_order(id).await,
            OrdersCommand::Modify => {
                println!("Modifying order");
            },
        },
        Rates(rates_cmd) => match rates_cmd {
            RatesCommand::Get => fetch_exchange_rates().await,
            RatesCommand::Set { rates } => set_exchange_rates(rates).await,
        },
    }
}

fn new_shopify_api() -> ShopifyApi {
    let config = ShopifyConfig::new_from_env_or_default();
    match ShopifyApi::new(config) {
        Ok(api) => api,
        Err(e) => {
            eprintln!("Error creating Shopify API: {e}");
            std::process::exit(1);
        },
    }
}

pub async fn fetch_shopify_order(id: u64) {
    let api = new_shopify_api();
    match api.get_order(id).await {
        Ok(order) => {
            let json = serde_json::to_string_pretty(&order).unwrap();
            println!("Order #{id}\n{json}");
        },
        Err(e) => {
            eprintln!("Error fetching order #{id}: {e}");
        },
    }
}

pub async fn cancel_shopify_order(id: u64) {
    let api = new_shopify_api();
    match api.cancel_order(id).await {
        Ok(order) => {
            let json = serde_json::to_string_pretty(&order).unwrap();
            println!("Cancelled order #{id}\n{json}");
        },
        Err(e) => {
            eprintln!("Error cancelling order #{id}: {e}");
        },
    }
}

pub async fn fetch_exchange_rates() {
    let api = new_shopify_api();
    match api.get_exchange_rates().await {
        Ok(rates) => {
            let json = serde_json::to_string_pretty(&rates).unwrap();
            println!("Exchange rates\n{json}");
        },
        Err(e) => {
            eprintln!("Error fetching exchange rates: {e}");
        },
    }
}

pub async fn set_exchange_rates(rates: Vec<ExchangeRate>) {
    let api = new_shopify_api();
    match api.set_exchange_rates(&rates).await {
        Ok(new_rates) => {
            println!("Exchange rates updated");
            let json = serde_json::to_string_pretty(&new_rates).unwrap();
            println!("New rates:\n{json}");
        },
        Err(e) => {
            eprintln!("Error updating exchange rates: {e}");
        },
    }
}
