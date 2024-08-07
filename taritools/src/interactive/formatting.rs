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
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::{Order, Payment, UserAccount},
    order_objects::{ClaimedOrder, OrderResult},
    tpe_api::{
        account_objects::{AccountAddress, CustomerId, FullAccount},
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

pub fn format_user_account(account: UserAccount) -> String {
    let mut table = Table::new();
    table.set_titles(row!["Field", "Value"]);
    table.add_row(Row::new(vec![Cell::new("ID"), Cell::new(&account.id.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Created At"), Cell::new(&account.created_at.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Updated At"), Cell::new(&account.updated_at.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Total Received"), Cell::new(&account.total_received.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Current Pending"), Cell::new(&account.current_pending.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Current Balance"), Cell::new(&account.current_balance.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Total Orders"), Cell::new(&account.total_orders.to_string())]));
    table.add_row(Row::new(vec![Cell::new("Current Orders"), Cell::new(&account.current_orders.to_string())]));

    // Format the table to a string
    markdown_style(&mut table);
    table.to_string()
}

pub fn format_addresses(addresses: &[AccountAddress]) -> String {
    let mut table = Table::new();
    table.set_titles(row!["Hex", "Emoji Id", "Created At", "Updated At"]);
    addresses.iter().for_each(|address| {
        let a = address.address.as_address();
        table.add_row(row![a.to_hex(), a.to_emoji_string(), address.created_at, address.updated_at]);
    });
    markdown_style(&mut table);
    table.to_string()
}

pub fn format_customer_ids(ids: &[CustomerId]) -> String {
    let mut table = Table::new();
    table.set_titles(row!["Customer id", "Created At", "Updated At"]);
    ids.iter().for_each(|id| {
        table.add_row(row![id.customer_id, id.created_at, id.updated_at]);
    });
    markdown_style(&mut table);
    table.to_string()
}

pub fn format_full_account(account: FullAccount) -> Result<String> {
    let mut s = String::new();
    writeln!(s, "# Account Summary")?;
    writeln!(s, "{}", format_user_account(account.account))?;
    writeln!(s, "# Addresses")?;
    writeln!(s, "{}\n", format_addresses(&account.addresses))?;
    writeln!(s, "# Customer IDs")?;
    writeln!(s, "{}\n", format_customer_ids(&account.customer_ids))?;
    writeln!(s, "# Orders")?;
    writeln!(s, "{}\n", format_orders(&account.orders))?;
    writeln!(s, "# Payments")?;
    writeln!(s, "{}\n", format_payments(&account.payments))?;
    Ok(s)
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
        "Order id: {id:15}          Created {created}",
        id = order.order_id.to_string(),
        created = order.created_at,
    )?;
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
    writeln!(f, "Payments for {});", payments.address.as_hex())?;
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
        Cell::new(&payment.sender.as_hex()),
        Cell::new(&payment.order_id.clone().map(|id| id.to_string()).unwrap_or_default()),
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
    let qr_link = format!("tari://{}/transactions/send?tariAddress={}", address.network(), address.to_hex());
    let code = QrCode::new(qr_link)
        .map(|code| {
            code.render::<unicode::Dense1x2>()
                .dark_color(unicode::Dense1x2::Dark)
                .light_color(unicode::Dense1x2::Light)
                .quiet_zone(false)
                .build()
        })
        .unwrap_or_default();
    (address.to_hex(), address.to_emoji_string(), code)
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
