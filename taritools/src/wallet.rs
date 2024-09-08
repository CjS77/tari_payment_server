use anyhow::{anyhow, Result};
use chrono::Utc;
use log::*;
use tari_payment_engine::{db_types::NewPayment, helpers::WalletSignature};
use tari_payment_server::data_objects::{
    PaymentNotification,
    TransactionConfirmation,
    TransactionConfirmationNotification,
};

use crate::{
    payments::{ConfirmationParams, ReceivedPaymentParams, WalletCommand},
    profile_manager::{read_config, Profile},
    tari_payment_server::client::PaymentServerClient,
};

pub async fn handle_wallet_command(command: WalletCommand) {
    let result = match command {
        WalletCommand::Received(params) => notify_server_about_payment(params).await,
        WalletCommand::Confirmed(params) => notify_server_about_confirmation(params).await,
    };
    if let Err(e) = result {
        error!("Wallet command failed: {e}");
        eprintln!("Wallet command failed: {e}")
    }
}

fn load_profile(name: &str) -> Result<Profile> {
    let user_data = read_config()?;
    let profile = user_data
        .profiles
        .iter()
        .find(|p| p.name == name)
        .cloned()
        .ok_or_else(|| anyhow!("Profile \"{name}\" not found"))?;
    Ok(profile)
}

fn new_nonce() -> i64 {
    Utc::now().timestamp_millis()
}

async fn notify_server_about_payment(params: ReceivedPaymentParams) -> Result<()> {
    let profile = load_profile(&params.profile)?;
    let client = PaymentServerClient::new(profile.clone());
    let payment = NewPayment::from(params);
    let key = profile.secret_key().ok_or_else(|| anyhow!("Profile {} is missing a secret key", profile.name))?;
    let auth = WalletSignature::create(profile.address, new_nonce(), &key, &payment)?;
    let notification = PaymentNotification { payment, auth };
    client.payment_notification(notification).await
}

async fn notify_server_about_confirmation(params: ConfirmationParams) -> Result<()> {
    let profile = load_profile(&params.profile)?;
    let txid = TransactionConfirmation { txid: params.txid };
    let client = PaymentServerClient::new(profile.clone());
    let key = profile.secret_key().ok_or_else(|| anyhow!("Profile {} is missing a secret key", profile.name))?;
    let auth = WalletSignature::create(profile.address, new_nonce(), &key, &txid)?;
    let confirmation = TransactionConfirmationNotification { confirmation: txid, auth };
    client.payment_confirmation(confirmation).await
}
