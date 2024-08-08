use anyhow::{anyhow, Result};
use clap::Subcommand;
use shopify_tools::ExchangeRate;
use tpg_common::MicroTari;

#[derive(Debug, Subcommand)]
pub enum ShopifyCommand {
    #[command(subcommand)]
    /// Retrieve or modify Shopify orders
    Orders(OrdersCommand),
    #[command(subcommand)]
    /// [Deprecated] Retrieve or modify exchange rates. Use the interactive mode of Taritools instead.
    Rates(RatesCommand),
    #[command(subcommand)]
    /// Retrieve or modify products
    Products(ProductsCommand),
    #[command(subcommand)]
    /// Configure or list webhooks
    Webhooks(WebhooksCommand),
}

#[derive(Debug, Subcommand)]
pub enum WebhooksCommand {
    /// Installs all necessary webhooks required for the Tari Payment server. If they already exist, they will be
    /// overwritten. The only parameter this command accepts is the URL of the server that will receive the webhook.
    /// If it is not provided, the TPG_HOST and TPG_PORT environment variables will be used to construct the URL.
    Install {
        #[arg(required = false, index = 1)]
        server_url: String,
    },
    /// List all webhooks installed on the Shopify store
    List,
}

#[derive(Debug, Subcommand)]
pub enum OrdersCommand {
    /// Fetch the order with the given ID
    Get {
        #[arg(required = true, index = 1)]
        id: u64,
    },
    /// Cancel the order with the given ID
    Cancel {
        #[arg(required = true, index = 1)]
        id: u64,
    },
    /// Mark the given order as paid on Shopify. This does not facilitate any transfer of funds; it only tells Shopify
    /// that the order has been paid for.
    Pay {
        #[arg(required = true, index = 1)]
        id: u64,
        #[arg(required = true, index = 2)]
        amount: String,
        #[arg(required = true, index = 3)]
        currency: String,
    },
    /// Modify the order
    Modify,
}

#[derive(Debug, Subcommand)]
pub enum ProductsCommand {
    /// Fetch all product variants with their Tari prices
    All,
    #[command(name = "update-price")]
    /// Updates prices for all products using the given exchange rates
    UpdatePrice {
        #[arg(required = true, index = 1)]
        /// The exchange rates to use for updating the prices in microTari per cent of the base currency
        microtari_per_cent: i64,
    },
    /// Retrieves product information for the given product variant ID
    Get {
        #[arg(required = true, index = 1)]
        id: u64,
    },
}

#[derive(Debug, Subcommand)]
pub enum RatesCommand {
    /// Fetch the current exchange rates
    Get,
    /// Modify the exchange rates
    Set {
        #[arg(required = true, index = 1, value_parser = parse_exchange_rate, value_delimiter = ',')]
        rates: Vec<ExchangeRate>,
    },
}

fn parse_exchange_rate(s: &str) -> Result<ExchangeRate> {
    let parts: Vec<&str> = s.split('=').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Exchange rate must be in the form 'USD=10'"));
    }
    let base_currency = parts[0].to_string();
    let rate = parts[1].parse::<i64>()?;
    let rate = MicroTari::from_tari(rate);
    Ok(ExchangeRate::new(base_currency, rate))
}
