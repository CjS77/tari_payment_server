use std::fmt::Write;

use prettytable::{
    format::{LinePosition, LineSeparator, TableFormat},
    row,
    Cell,
    Row,
    Table,
};
use tari_payment_engine::{
    db_types::{Order, Payment, UserAccount},
    order_objects::OrderResult,
    tpe_api::payment_objects::PaymentsResult,
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

pub fn format_order_result(orders: OrderResult) -> anyhow::Result<String> {
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
    orders.orders.iter().try_for_each(|order| format_order(order, &mut f))?;
    Ok(f)
}

pub fn format_orders(orders: Vec<Order>) -> anyhow::Result<String> {
    let mut f = String::new();
    writeln!(f, "===============================================================================")?;
    if orders.is_empty() {
        writeln!(f, "No open orders")?;
    } else {
        orders.iter().try_for_each(|order| format_order(order, &mut f))?;
    }
    writeln!(f, "===============================================================================")?;
    Ok(f)
}

pub fn format_order(order: &Order, f: &mut dyn Write) -> anyhow::Result<()> {
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

pub fn format_payments(payments: PaymentsResult) -> anyhow::Result<String> {
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
    let mut table = Table::new();
    table.set_titles(row!["TX id", "Amount", "Status", "Sender", "OrderID", "Memo", "Created At", "Updated At"]);
    payments.payments.iter().for_each(|payment| {
        table.add_row(payment_to_row(payment));
    });
    markdown_style(&mut table);
    f.write_str(&table.to_string())?;
    Ok(f)
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
