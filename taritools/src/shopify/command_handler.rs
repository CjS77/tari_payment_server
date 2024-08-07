use log::info;
use shopify_tools::{data_objects::Webhook, ExchangeRate, ShopifyApi, ShopifyApiError, ShopifyConfig};

use crate::shopify::{
    command_def::{ProductsCommand, RatesCommand, WebhooksCommand},
    OrdersCommand,
    ShopifyCommand,
};

pub async fn handle_shopify_command(command: ShopifyCommand) {
    use ShopifyCommand::*;
    match command {
        Orders(orders_command) => match orders_command {
            OrdersCommand::Get { id } => fetch_shopify_order(id).await,
            OrdersCommand::Cancel { id } => cancel_shopify_order(id).await,
            OrdersCommand::Pay { id, amount, currency } => mark_order_as_paid(id, amount, currency).await,
            OrdersCommand::Modify => {
                println!("Modifying order");
            },
        },
        Rates(rates_cmd) => match rates_cmd {
            RatesCommand::Get => fetch_exchange_rates().await,
            RatesCommand::Set { rates } => set_exchange_rates(rates).await,
        },
        Products(products_cmd) => match products_cmd {
            ProductsCommand::All => fetch_all_variants().await,
            ProductsCommand::UpdatePrice { microtari_per_cent } => update_prices(microtari_per_cent).await,
            ProductsCommand::Get { id } => get_variant(id).await,
        },
        Webhooks(cmd) => match cmd {
            WebhooksCommand::Install { server_url } => install_webhooks(server_url).await,
            WebhooksCommand::List => list_webhooks().await,
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
            let json = serde_json::to_string_pretty(&order)
                .unwrap_or_else(|e| format!("Could not represent order as JSON. {e}"));
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
            let json = serde_json::to_string_pretty(&order)
                .unwrap_or_else(|e| format!("Could not represent order as JSON. {e}"));
            println!("Cancelled order #{id}\n{json}");
        },
        Err(e) => {
            eprintln!("Error cancelling order #{id}: {e}");
        },
    }
}

pub async fn mark_order_as_paid(id: u64, amount: String, currency: String) {
    let api = new_shopify_api();
    match api.mark_order_as_paid(id, amount, currency).await {
        Ok(tx) => {
            let json = serde_json::to_string_pretty(&tx)
                .unwrap_or_else(|e| format!("Could not represent transaction as JSON. {e}"));
            println!("Marked order #{id} as paid\n{json}");
        },
        Err(e) => {
            eprintln!("Error marking order #{id} as paid: {e}");
        },
    }
}

pub async fn fetch_exchange_rates() {
    let api = new_shopify_api();
    match api.get_exchange_rates().await {
        Ok(rates) => {
            let json = serde_json::to_string_pretty(&rates)
                .unwrap_or_else(|e| format!("Could not represent rate as JSON. {e}"));
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
            let json = serde_json::to_string_pretty(&new_rates)
                .unwrap_or_else(|e| format!("Could not represent rates as JSON. {e}"));
            println!("New rates:\n{json}");
        },
        Err(e) => {
            eprintln!("Error updating exchange rates: {e}");
        },
    }
}

pub async fn fetch_all_variants() {
    let api = new_shopify_api();
    match api.fetch_all_variants().await {
        Ok(variants) => {
            let json = serde_json::to_string_pretty(&variants)
                .unwrap_or_else(|e| format!("Could not represent product as JSON. {e}"));
            println!("Variants\n{json}");
        },
        Err(e) => {
            eprintln!("Error fetching variants: {e}");
        },
    }
}

pub async fn get_variant(id: u64) {
    let api = new_shopify_api();
    match api.fetch_variant(id).await {
        Ok(variant) => {
            let json = serde_json::to_string_pretty(&variant)
                .unwrap_or_else(|e| format!("Could not represent product as JSON. {e}"));
            println!("Variant #{id}\n{json}");
        },
        Err(ShopifyApiError::EmptyResponse) => {
            println!("Variant #{id} not found");
        },
        Err(e) => {
            eprintln!("Error fetching variant #{id}: {e}");
        },
    }
}

pub async fn update_prices(rate: i64) {
    let api = new_shopify_api();
    let rate = ExchangeRate::new("USD".to_string(), rate.into());
    match api.update_all_prices(rate).await {
        Ok(variants) => {
            println!("Prices updated");
            let json = serde_json::to_string_pretty(&variants)
                .unwrap_or_else(|e| format!("Could not represent product as JSON. {e}"));
            println!("Variants:\n{json}");
        },
        Err(e) => {
            eprintln!("Error updating prices: {e}");
        },
    }
}

pub async fn list_webhooks() {
    let api = new_shopify_api();
    match api.fetch_webhooks().await {
        Ok(webhooks) => {
            let json = serde_json::to_string_pretty(&webhooks)
                .unwrap_or_else(|e| format!("Could not represent webhook as JSON. {e}"));
            println!("Webhooks\n{json}");
        },
        Err(e) => {
            eprintln!("Error listing webhooks: {e}");
        },
    }
}

async fn install_webhooks(url: String) {
    let api = new_shopify_api();
    let make_address = |topic| format!("{url}/shopify/webhook/{topic}");
    let existing_webhooks = match api.fetch_webhooks().await {
        Ok(webhooks) => webhooks,
        Err(e) => {
            eprintln!("Error fetching existing webhooks: {e}");
            return;
        },
    };
    let params =
        [("orders/create", make_address("checkout_create")), ("products/update", make_address("product_updated"))];
    for (topic, address) in params {
        match in_existing(topic, &existing_webhooks) {
            Some(webhook) => {
                if webhook.address == address {
                    println!("Webhook already exists for {topic}. Skipping");
                } else {
                    info!("Webhook already exists for {topic}. Updating address");
                    match api.update_webhook(webhook.id, &address).await {
                        Ok(webhook) => {
                            println!("Webhook address updated from {} to {} for {topic}", webhook.address, address);
                        },
                        Err(e) => {
                            eprintln!("Error updating webhook for {topic}: {e}");
                        },
                    }
                }
            },
            None => match api.install_webhook(&address, topic).await {
                Ok(webhook) => {
                    println!("Webhook installed for {topic}");
                    let json = serde_json::to_string_pretty(&webhook)
                        .unwrap_or_else(|e| format!("Could not represent webhook as JSON. {e}"));
                    println!("Webhook:\n{json}");
                },
                Err(e) => {
                    eprintln!("Error installing webhook for {topic}: {e}");
                },
            },
        }
    }
}

fn in_existing<'a>(topic: &str, webhooks: &'a [Webhook]) -> Option<&'a Webhook> {
    webhooks.iter().find(|w| w.topic == topic)
}
