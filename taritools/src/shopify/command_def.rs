use clap::Subcommand;
use shopify_tools::ExchangeRate;
use tpg_common::MicroTari;

#[derive(Debug, Subcommand)]
pub enum ShopifyCommand {
    #[command(subcommand)]
    /// Retrieve or modify Shopify orders
    Orders(OrdersCommand),
    #[command(subcommand)]
    /// Retrieve or modify exchange rates
    Rates(RatesCommand),
    #[command(subcommand)]
    /// Retrieve or modify products
    Products(ProductsCommand),
}

#[derive(Debug, Subcommand)]
pub enum OrdersCommand {
    /// Fetch the order with the given ID
    Get {
        #[arg(required = true, index = 1)]
        id: u64,
    },
    /// Cacnel the order with the given ID
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

fn parse_exchange_rate(s: &str) -> Result<ExchangeRate, String> {
    let parts: Vec<&str> = s.split('=').collect();
    if parts.len() != 2 {
        return Err("Exchange rate must be in the form 'USD=10'".to_string());
    }
    let base_currency = parts[0].to_string();
    let rate = parts[1].parse::<i64>().map_err(|e| e.to_string())?;
    let rate = MicroTari::from_tari(rate);
    Ok(ExchangeRate::new(base_currency, rate))
}
