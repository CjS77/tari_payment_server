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
    /// Modify the order
    Modify,
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
