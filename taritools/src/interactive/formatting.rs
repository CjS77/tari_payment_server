use std::fmt::Write;

use anyhow::Result;
use prettytable::{
    format::{LinePosition, LineSeparator, TableFormat},
    row,
    Cell,
    Row,
    Table,
};
use qrcode::{render::unicode, QrCode};
use shopify_tools::ShopifyOrder;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::{
        AddressBalance,
        CustomerBalance,
        CustomerOrderBalance,
        CustomerOrders,
        Order,
        Payment,
        SettlementJournalEntry,
    },
    order_objects::{ClaimedOrder, OrderResult},
    tpe_api::{
        account_objects::{AddressHistory, CustomerHistory},
        payment_objects::PaymentsResult,
    },
    traits::WalletInfo,
};
use tari_payment_server::data_objects::ExchangeRateResult;
use tpg_common::MicroTari;

fn markdown_format() -> TableFormat {
    prettytable::format::FormatBuilder::new()
        .column_separator('|')
        .borders('|')
        .separator(LinePosition::Title, LineSeparator::new('-', '|', '|', '|'))
        .padding(1, 1)
        .build()
}

fn markdown_style(table: &mut Table) {
    table.set_format(markdown_format());
}

pub fn format_address_balance(balance: &AddressBalance) -> Result<String> {
    let mut f = String::new();
    writeln!(f, "Address: {}", balance.address())?;
    writeln!(f, "Available: {}", balance.current_balance())?;
    writeln!(f, "Total received: {}", balance.total_confirmed())?;
    writeln!(f, "Total spent: {}", balance.total_paid())?;
    Ok(f)
}

pub fn format_order_result(orders: OrderResult) -> Result<String> {
    let mut f = String::new();
    writeln!(f, "===============================================================================")?;
    writeln!(
        f,
        "Orders for {address}\n{count:>4} orders. Total value: {value}",
        count = orders.orders.len(),
        address = orders.address,
        value = orders.total_orders
    )?;
    writeln!(f, "===============================================================================")?;
    writeln!(f, "{}", format_orders(&orders.orders))?;
    Ok(f)
}

pub fn format_orders(orders: &[Order]) -> String {
    if orders.is_empty() {
        return "No open orders".to_string();
    }
    let mut table = Table::new();
    table.set_titles(row![
        "ID",
        "Order id",
        "Alt id",
        "Customer id",
        "Amount",
        "Status",
        "Original price",
        "Currency",
        "Memo",
        "Created At",
        "Updated At"
    ]);
    let mut memos = Vec::new();
    orders.iter().for_each(|order| {
        let memo_note = match order.memo {
            Some(ref memo) => {
                memos.push(memo.clone());
                format!("{}^", memos.len())
            },
            None => String::default(),
        };
        table.add_row(row![
            order.id,
            order.order_id,
            order.alt_id.as_ref().map(|o| o.to_string()).unwrap_or_default(),
            order.customer_id,
            order.total_price.to_string(),
            order.status.to_string(),
            order.original_price.as_deref().unwrap_or_default(),
            order.currency,
            memo_note,
            order.created_at.to_string(),
            order.updated_at.to_string()
        ]);
    });
    markdown_style(&mut table);
    let memo_notes =
        memos.iter().enumerate().map(|(i, memo)| format!("^{}: {}", i + 1, memo)).collect::<Vec<String>>().join("\n");
    if memo_notes.is_empty() {
        format!("{table}\n")
    } else {
        format!("{table}\n## Memos\n{memo_notes}")
    }
}

pub fn format_order(order: &Order, f: &mut dyn Write) -> Result<()> {
    writeln!(
        f,
        "Order id: {id:15}{name:9} Created {created}",
        id = order.order_id.to_string(),
        name = order.alt_id.as_ref().map(|s| format!("({})", s)).unwrap_or_default(),
        created = order.created_at,
    )?;
    writeln!(f, "Customer id: {}", order.customer_id)?;
    writeln!(f, "[{:^15}]                  Updated {}", order.status.to_string(), order.updated_at)?;
    writeln!(f, "-----------------------------------------------------------------------------")?;
    writeln!(f, "Total Price:    {total}", total = order.total_price)?;
    writeln!(
        f,
        "Original Price: {original} {currency}",
        original = order.original_price.as_deref().unwrap_or("Not given"),
        currency = order.currency
    )?;
    writeln!(f, "Memo: {memo}", memo = order.memo.as_deref().unwrap_or("No memo"))?;
    writeln!(f, "-----------------------------------------------------------------------------\n")?;
    Ok(())
}

pub fn format_shopify_orders(orders: &[ShopifyOrder]) -> String {
    if orders.is_empty() {
        return "No open orders".to_string();
    }
    let mut table = Table::new();
    table.set_titles(row!["id", "name", "Customer id", "Total price", "Cur", "Note", "Created At", "Updated At"]);
    let mut memos = Vec::new();
    orders.iter().for_each(|order| {
        let memo_note = match order.note {
            Some(ref memo) => {
                memos.push(memo.clone());
                format!("{}^", memos.len())
            },
            None => String::default(),
        };
        table.add_row(row![
            order.id,
            order.name,
            order.customer.id.to_string(),
            format!("{:>11}", order.total_price),
            order.currency,
            memo_note,
            order.created_at,
            order.updated_at
        ]);
    });
    markdown_style(&mut table);
    let memo_notes =
        memos.iter().enumerate().map(|(i, memo)| format!("^{}: {}", i + 1, memo)).collect::<Vec<String>>().join("\n");
    if memo_notes.is_empty() {
        format!("{table}\n")
    } else {
        format!("{table}\n## Memos\n{memo_notes}")
    }
}

pub fn format_wallet_list(wallets: &[WalletInfo]) -> String {
    let mut table = Table::new();
    table.set_titles(row!["Address", "Emoji ID", "IP Address", "Last nonce"]);
    wallets.iter().for_each(|wallet| {
        table.add_row(row![
            wallet.address,
            wallet.address.as_address().to_emoji_string(),
            wallet.ip_address,
            wallet.last_nonce,
        ]);
    });
    markdown_style(&mut table);
    table.to_string()
}

pub fn format_payments_result(payments: PaymentsResult) -> Result<String> {
    let mut f = String::new();
    writeln!(f, "===============================================================================")?;
    writeln!(f, "Payments for {});", payments.address.as_base58())?;
    writeln!(
        f,
        "{count:<4} payments.                                          Total value: {value}",
        count = payments.payments.len(),
        value = payments.total_payments
    )?;
    writeln!(f, "===============================================================================")?;
    f.write_str(&format_payments(&payments.payments))?;
    Ok(f)
}

pub fn format_payments(payments: &[Payment]) -> String {
    let mut table = Table::new();
    table.set_titles(row!["TX id", "Amount", "Status", "Sender", "OrderID", "Memo", "Created At", "Updated At"]);
    payments.iter().for_each(|payment| {
        table.add_row(payment_to_row(payment));
    });
    markdown_style(&mut table);
    table.to_string()
}

pub fn payment_to_row(payment: &Payment) -> Row {
    Row::new(vec![
        Cell::new(&payment.txid),
        Cell::new(&payment.amount.to_string()),
        Cell::new(&payment.status.to_string()),
        Cell::new(&payment.sender.as_base58()),
        Cell::new(&payment.order_id.as_ref().map(|o| o.to_string()).unwrap_or_default()),
        Cell::new(&payment.memo.clone().unwrap_or_default()),
        Cell::new(&payment.created_at.to_string()),
        Cell::new(&payment.updated_at.to_string()),
    ])
}

pub fn format_exchange_rate(rate: ExchangeRateResult) -> String {
    let tari = MicroTari::from(rate.rate);
    format!("1 {} => {tari} (Last update: {})", rate.currency, rate.updated_at)
}

pub fn print_order(order: &Order) -> Result<String> {
    let mut f = String::new();
    format_order(order, &mut f)?;
    Ok(f)
}

pub fn format_addresses_with_qr_code(addresses: &[TariAddress]) -> String {
    let mut table = Table::new();
    table.set_titles(row!["Hex", "Emoji Id", "QR Code"]);
    addresses.iter().for_each(|a| {
        let (hex, emoji, qr) = format_address_with_qr_code(a);
        table.add_row(row![hex, emoji, qr]);
    });
    markdown_style(&mut table);
    table.to_string()
}

pub fn format_address_with_qr_code(address: &TariAddress) -> (String, String, String) {
    let qr_link = format!("tari://{}/transactions/send?tariAddress={}", address.network(), address.to_base58());
    let code = QrCode::new(qr_link)
        .map(|code| {
            code.render::<unicode::Dense1x2>()
                .dark_color(unicode::Dense1x2::Dark)
                .light_color(unicode::Dense1x2::Light)
                .quiet_zone(false)
                .build()
        })
        .unwrap_or_default();
    (address.to_base58(), address.to_emoji_string(), code)
}

pub fn format_claimed_order(order: &ClaimedOrder) -> Result<String> {
    let mut f = String::new();
    writeln!(f, "## Claimed order details")?;
    writeln!(f, "Order id: {:<25} Status: {}", order.order_id.as_str(), order.status)?;
    writeln!(f, "Amount: {}", order.total_price)?;
    writeln!(f, "Payment due before: {}", order.expires_at)?;
    let (hex, emoji, qr) = format_address_with_qr_code(&order.send_to);
    writeln!(f, "Send Payment to: {hex} ({emoji})")?;
    writeln!(f, "{qr}")?;
    Ok(f)
}

pub fn format_address_history(history: &AddressHistory) -> Result<String> {
    let mut f = String::new();
    writeln!(f, "## History for {}", history.address.as_base58())?;
    let balances = format_address_balance(&history.balance)?;
    writeln!(f, "### Balances\n\n{balances}\n")?;
    let orders_str = format_orders(&history.orders);
    writeln!(f, "### Associated orders\n\n{orders_str}\n")?;
    let payments = format_payments(&history.payments);
    writeln!(f, "### Payments\n\n{payments}\n")?;
    let settlements = format_settlements(&history.settlements)?;
    writeln!(f, "### Transactions: {settlements}")?;
    Ok(f)
}

pub fn format_customer_balance(balance: &CustomerBalance) -> Result<String> {
    let mut f = String::new();
    writeln!(f, "Total transfers confirmed: {}", balance.total_confirmed())?;
    writeln!(f, "Total paid: {}", balance.total_paid())?;
    writeln!(f, "Available balance: {}", balance.current_balance())?;
    writeln!(f, "Associated wallet addresses")?;
    for address in balance.addresses() {
        let balance = format_address_balance(address)?;
        writeln!(f, "{balance}\n")?;
    }
    Ok(f)
}

pub fn format_customer_order_balance(order_balance: &CustomerOrderBalance) -> Result<String> {
    let mut f = String::new();
    writeln!(f, "Total current orders: {}", order_balance.total_current)?;
    writeln!(f, "Total paid orders: {}", order_balance.total_paid)?;
    writeln!(f, "Total expired orders: {}", order_balance.total_expired)?;
    writeln!(f, "Total cancelled orders: {}", order_balance.total_cancelled)?;
    Ok(f)
}

pub fn format_customer_history(history: &CustomerHistory) -> Result<String> {
    let mut f = String::new();
    writeln!(f, "## History for Customer [{}]", history.customer_id)?;
    let balance = format_customer_balance(&history.balance)?;
    writeln!(f, "### Balances\n\n{balance}\n")?;
    let order_balance = format_customer_order_balance(&history.order_balance)?;
    writeln!(f, "### Order Balances\n\n{order_balance}\n")?;
    let orders = format_orders(&history.orders);
    writeln!(f, "### Orders\n\n{orders}\n")?;
    let settlements = format_settlements(&history.settlements)?;
    writeln!(f, "### Transactions:\n\n{settlements}\n")?;
    Ok(f)
}

pub fn format_customer_orders(orders: &[CustomerOrders]) -> Result<String> {
    let mut f = String::new();
    let mut table = Table::new();
    table.set_titles(row!["Customer ID", "Order status", "Total value"]);
    orders.iter().for_each(|customer| {
        table.add_row(row![customer.customer_id, customer.status, customer.total_orders.to_string(),]);
    });
    markdown_style(&mut table);
    writeln!(f, "{table}")?;
    Ok(f)
}

pub fn format_settlements(settlements: &[SettlementJournalEntry]) -> Result<String> {
    let mut f = String::new();
    let mut table = Table::new();
    table.set_titles(row!["Timestamp", "Order id", "Amount", "Type", "From Address"]);
    settlements.iter().for_each(|settlement| {
        table.add_row(row![
            settlement.created_at,
            settlement.order_id,
            settlement.amount,
            settlement.settlement_type,
            settlement.payment_address.as_base58()
        ]);
    });
    markdown_style(&mut table);
    writeln!(f, "{table}")?;
    Ok(f)
}
