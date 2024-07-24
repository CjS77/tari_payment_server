use std::fmt::Display;

use anyhow::{anyhow, Result};
use chrono::Utc;
use log::info;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
    StatusCode,
};
use serde::de::DeserializeOwned;
use tari_common_types::tari_address::TariAddress;
use tari_jwt::{
    jwt_compact::{AlgorithmExt, Claims, Header},
    Ristretto256,
    Ristretto256SigningKey,
};
use tari_payment_engine::{
    db_types::{CreditNote, LoginToken, Order, Role, UserAccount},
    order_objects::OrderResult,
    tpe_api::payment_objects::PaymentsResult,
};
use tari_payment_server::data_objects::{
    ExchangeRateResult,
    ExchangeRateUpdate,
    PaymentNotification,
    TransactionConfirmationNotification,
};
use tpg_common::MicroTari;

use crate::profile_manager::Profile;

pub struct PaymentServerClient {
    client: Client,
    profile: Profile,
    access_token: String,
}

impl PaymentServerClient {
    pub fn new(profile: Profile) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let client = Client::builder()
            .user_agent("Tari Payment Server Client")
            .default_headers(headers)
            .build()
            .expect("Failed to create reqwest client");
        PaymentServerClient { client, profile, access_token: "".to_string() }
    }

    pub fn server(&self) -> &str {
        &self.profile.server
    }

    pub fn profile_name(&self) -> &str {
        &self.profile.name
    }

    pub fn roles(&self) -> Vec<Role> {
        self.profile.roles.clone()
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.profile.server, path)
    }

    pub async fn authenticate(&mut self) -> Result<()> {
        println!("Authenticating with payment server");
        let token = generate_auth_token(&self.profile)?;
        let url = self.url("/auth");
        let res = self.client.post(url).header("tpg_auth_token", token).send().await?;
        if !res.status().is_success() {
            let reason = res.text().await?;
            return Err(anyhow::anyhow!("Failed to authenticate with payment server. {reason}"));
        }
        let token: String = res.text().await?;
        info!("Authenticated with payment server. Access token: {}******", &token[..12]);
        self.access_token = token;
        Ok(())
    }

    pub async fn health(&self) -> Result<String> {
        let url = self.url("/health");
        let res = self.client.get(url).send().await?;
        let response = res.text().await?;
        Ok(response)
    }

    pub async fn my_account(&self) -> Result<UserAccount> {
        let url = self.url("/api/account");
        let res = self.client.get(url).header("tpg_access_token", self.access_token.clone()).send().await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            StatusCode::NOT_FOUND => Err(anyhow!("No account exists for you yet")),
            _ => {
                let msg = res.text().await?;
                Err(anyhow!("Error fetching account: {msg}"))
            },
        }
    }

    pub async fn fetch_exchange_rates(&self, currency: &str) -> Result<ExchangeRateResult> {
        self.auth_get_request(&format!("/api/exchange_rate/{currency}")).await
    }

    pub async fn set_exchange_rate(&self, currency: &str, price_in_tari: MicroTari) -> Result<()> {
        let url = self.url("/api/exchange_rate");
        let rate = ExchangeRateUpdate { currency: currency.to_string(), rate: price_in_tari.value() as u64 };
        let res =
            self.client.post(url).header("tpg_access_token", self.access_token.clone()).json(&rate).send().await?;
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error setting exchange rates: {msg}"));
        }
        Ok(())
    }

    /// Send a payment notification to the payment server.
    pub async fn payment_notification(&self, notification: PaymentNotification) -> Result<()> {
        let url = self.url("/wallet/incoming_payment");
        let res = self.client.post(url).json(&notification).send().await?;
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error sending payment notification: {msg}"));
        }
        Ok(())
    }

    /// Send a payment confirmation to the payment server.
    pub async fn payment_confirmation(&self, confirmation: TransactionConfirmationNotification) -> Result<()> {
        let url = self.url("/wallet/tx_confirmation");
        let res = self.client.post(url).json(&confirmation).send().await?;
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error sending payment confirmation: {msg}"));
        }
        Ok(())
    }

    pub async fn my_orders(&self) -> Result<OrderResult> {
        self.auth_get_request("/api/orders").await
    }

    pub async fn my_unfulfilled_orders(&self) -> Result<Vec<Order>> {
        self.auth_get_request("/api/unfulfilled_orders").await
    }

    pub async fn my_payments(&self) -> Result<PaymentsResult> {
        self.auth_get_request("/api/payments").await
    }

    async fn auth_get_request<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.url(path);
        let res = self.client.get(url).header("tpg_access_token", self.access_token.clone()).send().await?;
        match res.status() {
            StatusCode::OK => Ok({
                res.json().await?
                // let body: Value = res.json().await?;
                // println!("body: {:?}", body);
                // serde_json::from_value(body).unwrap()
            }),
            code => {
                let msg = res.text().await?;
                Err(anyhow!("Error fetching {path}: {code}, {msg}."))
            },
        }
    }

    pub async fn customer_ids(&self) -> Result<Vec<String>> {
        self.auth_get_request("/api/customer_ids").await
    }

    pub async fn addresses(&self) -> Result<Vec<String>> {
        self.auth_get_request("/api/addresses").await
    }

    pub async fn orders_for_address(&self, address: TariAddress) -> Result<OrderResult> {
        self.auth_get_request(&format!("/api/orders/{}", address.to_hex())).await
    }

    pub async fn order_by_id(&self, order_id: &str) -> Result<Option<Order>> {
        self.auth_get_request(&format!("/api/order/id/{order_id}")).await
    }

    pub async fn payments_for_address(&self, address: TariAddress) -> Result<PaymentsResult> {
        self.auth_get_request(&format!("/api/payments/{}", address.to_hex())).await
    }

    pub async fn issue_credit(&self, customer_id: &str, amount: MicroTari, reason: String) -> Result<Vec<Order>> {
        let url = self.url("/api/credit");
        let credit = CreditNote::new(customer_id.to_string(), amount).with_reason(reason);
        let res =
            self.client.post(url).header("tpg_access_token", self.access_token.clone()).json(&credit).send().await?;
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error issuing credit: {msg}"));
        }
        let paid_orders = res.json().await?;
        Ok(paid_orders)
    }
}

impl Display for PaymentServerClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.profile_name())?;
        let roles = self.roles().iter().map(|r| format!("{}", r)).collect::<Vec<String>>().join(", ");
        write!(f, " [{roles}]")?;
        write!(f, " ({})", self.server())
    }
}

fn generate_auth_token(profile: &Profile) -> Result<String> {
    let nonce = Utc::now().timestamp() as u64;
    let address = profile.address.clone().to_address();
    let claims = LoginToken { address, nonce, desired_roles: profile.roles.clone() };
    let claims = Claims::new(claims);
    let header = Header::empty().with_token_type("JWT");
    let key = profile.secret_key().ok_or_else(|| anyhow!("Profile {} is missing a secret key", profile.name))?;
    let token = Ristretto256.token(&header, &claims, &Ristretto256SigningKey(key))?;
    Ok(token)
}
