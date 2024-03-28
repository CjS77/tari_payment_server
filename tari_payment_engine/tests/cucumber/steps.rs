use std::{str::FromStr, time::Duration};

use cucumber::{then, when};
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::{MicroTari, NewOrder, NewPayment, OrderId, OrderStatusType, OrderUpdate, TransferStatus, UserAccount},
    AccountManagement,
    PaymentGatewayDatabase,
};

use crate::cucumber::ShopifyWorld;

#[when(expr = "I receive an order with id {word} from customer '{word}' for {int} XTR")]
async fn receive_order(world: &mut ShopifyWorld, order_id: String, customer_id: String, price: i64) {
    let id = OrderId::from(order_id);
    let order = NewOrder::new(id, customer_id, MicroTari::from(price * 1_000_000));
    let _res = world.api().process_new_order(order).await.expect("Error processing order");
}

#[when(expr = "I receive a wallet payment with txid [{word}] from '{word}' for {int} XTR")]
async fn receive_wallet_payment(world: &mut ShopifyWorld, txid: String, pubkey: String, amount: i64) {
    wallet_payment(world, txid, pubkey, amount, None).await;
}

//             I receive a wallet payment with txid [tari_tx2] from 'pk4Bob' for 65 XTR and memo 'order id: 200'
#[when(expr = "I receive a wallet payment with txid [{word}] from '{word}' for {int} XTR and memo {string}")]
async fn wallet_payment_with_memo(world: &mut ShopifyWorld, txid: String, pubkey: String, amount: i64, memo: String) {
    wallet_payment(world, txid, pubkey, amount, Some(memo)).await;
}

async fn wallet_payment(world: &mut ShopifyWorld, txid: String, pubkey: String, amount: i64, memo: Option<String>) {
    let amount = MicroTari::from(amount * 1_000_000);
    let pk = TariAddress::from_str(&pubkey).expect("Not a valid Tari address");
    let mut payment = NewPayment::new(pk, amount, txid);
    if let Some(memo) = memo {
        payment = payment.with_memo(memo);
    }
    let _res = world.api().process_new_payment(payment).await.expect("Error processing payment");
}

#[when(expr = "payment [{word}] confirms")]
async fn confirm_payment(world: &mut ShopifyWorld, txid: String) {
    let _res = world.api().confirm_transaction(txid).await.expect("Error confirming payment");
}

#[when(expr = "payment [{word}] is cancelled")]
async fn cancel_payment(world: &mut ShopifyWorld, txid: String) {
    world.api().cancel_transaction(txid).await.expect("Error cancelling payment");
}

#[when(expr = "I pause for {int}ms")]
async fn pause(_world: &mut ShopifyWorld, ms: u64) {
    let delay = Duration::from_millis(ms);
    tokio::time::sleep(delay).await;
}

async fn fetch_customer_account(world: &mut ShopifyWorld, cust_id: &str) -> Option<UserAccount> {
    let db = world.api().db();
    db.fetch_user_account_for_customer_id(cust_id).await.expect("Error fetching account")
}

#[then(expr = "the account for customer '{word}' exists")]
async fn check_account_exists_for_customer(world: &mut ShopifyWorld, cust_id: String) {
    let account = fetch_customer_account(world, &cust_id).await;
    assert!(account.is_some(), "Account does not exist");
}

#[then(expr = "the account for address '{word}' exists")]
async fn check_account_exists_for_pubkey(world: &mut ShopifyWorld, pubkey: String) {
    let db = world.api().db();
    let pubkey = TariAddress::from_str(&pubkey).expect("Not a valid Tari address. {pubkey}");
    let account = db.fetch_user_account_for_address(&pubkey).await.expect("Error fetching account");
    assert!(account.is_some(), "Account does not exist");
}

#[then(expr = "the account for customer '{word}' has total orders of {int} XTR")]
async fn check_account_total_orders(world: &mut ShopifyWorld, customer_id: String, value: i64) {
    let account = fetch_customer_account(world, &customer_id).await.expect("Account for {customer_id} does not exist");
    assert_eq!(account.total_orders, MicroTari::from(value * 1_000_000), "Total orders is incorrect");
}

#[then(expr = "the account for customer '{word}' has total received of {int} XTR")]
async fn check_customer_total_received(world: &mut ShopifyWorld, customer_id: String, value: i64) {
    let account = fetch_customer_account(world, &customer_id).await.expect("Account for {customer_id} does not exist");
    assert_eq!(account.total_received, MicroTari::from(value * 1_000_000), "Total received is incorrect");
}

#[then(expr = "the account for customer '{word}' has total pending of {int} XTR")]
async fn check_customer_total_pending(world: &mut ShopifyWorld, customer_id: String, value: i64) {
    let account = fetch_customer_account(world, &customer_id).await.expect("Account for {customer_id} does not exist");
    assert_eq!(account.total_pending, MicroTari::from(value * 1_000_000), "Total received is incorrect");
}

#[then(expr = "the account for customer '{word}' has current balance of {int} XTR")]
async fn check_customer_balance(world: &mut ShopifyWorld, customer_id: String, value: i64) {
    let account = fetch_customer_account(world, &customer_id).await.expect("Account for {customer_id} does not exist");
    assert_eq!(account.current_balance, MicroTari::from(value * 1_000_000), "Current balance is incorrect");
}

async fn fetch_account_for_address(world: &mut ShopifyWorld, pubkey: String) -> Option<UserAccount> {
    let db = world.api().db();
    let pubkey = TariAddress::from_str(&pubkey).expect("Not a valid Tari address. {pubkey}");
    db.fetch_user_account_for_address(&pubkey).await.expect("Error fetching account {pubkey}")
}

#[then(expr = "the account for address '{word}' has total received of {int} XTR")]
async fn check_account_total_received(world: &mut ShopifyWorld, pubkey: String, value: i64) {
    let account = fetch_account_for_address(world, pubkey).await.expect("Account {pubkey} does not exist");
    assert_eq!(account.total_received, MicroTari::from(value * 1_000_000), "Total received is incorrect");
}

#[then(expr = "the account for address '{word}' has total pending of {int} XTR")]
async fn check_account_total_pending(world: &mut ShopifyWorld, pubkey: String, value: i64) {
    let account = fetch_account_for_address(world, pubkey).await.expect("Account {pubkey} does not exist");
    assert_eq!(account.total_pending, MicroTari::from(value * 1_000_000), "Total pending is incorrect");
}

#[then(expr = "the account for address '{word}' has current balance of {int} XTR")]
async fn check_account_current_balance(world: &mut ShopifyWorld, pubkey: String, value: i64) {
    let account = fetch_account_for_address(world, pubkey).await.expect("Account {pubkey} does not exist");
    assert_eq!(account.current_balance, MicroTari::from(value * 1_000_000), "Current balance is incorrect");
}

#[then(expr = "the order with id {word} has {word} of '{word}'")]
async fn order_status(world: &mut ShopifyWorld, order_id: OrderId, field: String, value: String) {
    let order = world
        .api()
        .db()
        .order_by_order_id(&order_id)
        .await
        .expect("Error fetching order")
        .expect("Order {order_id} does not exist");
    match field.as_str() {
        "status" => assert_eq!(order.status, OrderStatusType::from(value), "Status is incorrect"),
        "customer_id" => assert_eq!(order.customer_id, value, "Customer ID is incorrect"),
        "total_price" => {
            let price = value.parse::<i64>().expect("Invalid price") * 1_000_000;
            let price = MicroTari::from(price);
            assert_eq!(order.total_price, price, "Total price is incorrect")
        },
        "currency" => assert_eq!(order.currency, value, "Currency is incorrect"),
        _ => panic!("Unknown field {field}"),
    }
}

#[when(expr = "order {word} is updated with {word} of '{word}'")]
async fn update_order(world: &mut ShopifyWorld, oid: OrderId, field: String, value: String) {
    let mut update = OrderUpdate::default();
    match field.as_str() {
        "status" => update = update.with_status(OrderStatusType::from(value)),
        "memo" => update = update.with_memo(value),
        "total_price" => {
            let price = value.parse::<i64>().expect("Invalid price") * 1_000_000;
            update = update.with_total_price(MicroTari::from(price));
        },
        "currency" => update = update.with_currency(value),
        _ => panic!("Unknown field {field}"),
    }
    let db = world.api().db();
    db.update_order(&oid, update).await.expect("Error updating order");
}

#[then(expr = "the confirmation status for payment #{int} is {string}")]
async fn check_confirmation_status(_world: &mut ShopifyWorld, _payment_index: u64, status: String) {
    let _expected = TransferStatus::from(status);
    todo!("update this");
    // let payment_status = db
    //     .payment_status_for(payment_index)
    //     .await
    //     .expect("Error fetching payment status")
    //     .expect("No entry found for payment status");
    // assert_eq!(
    //     payment_status.transfer_status, expected,
    //     "Transfer status is incorrect"
    // );
}
