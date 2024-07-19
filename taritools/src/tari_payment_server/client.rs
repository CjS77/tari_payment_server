use anyhow::{anyhow, Result};
use chrono::Utc;
use log::info;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
    StatusCode,
};
use tari_jwt::{
    jwt_compact::{AlgorithmExt, Claims, Header},
    Ristretto256,
    Ristretto256SigningKey,
};
use tari_payment_engine::db_types::{LoginToken, UserAccount};
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
        let url = self.url(format!("/api/exchange_rate/{currency}").as_str());
        let res = self.client.get(url).header("tpg_access_token", self.access_token.clone()).send().await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            code => {
                let msg = res.text().await?;
                Err(anyhow!("Error fetching exchange rate: {code}, {msg}."))
            },
        }
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
}

fn generate_auth_token(profile: &Profile) -> Result<String> {
    let nonce = Utc::now().timestamp() as u64;
    let address = profile.address.clone().to_address();
    let claims = LoginToken { address, nonce, desired_roles: profile.roles.clone() };
    let claims = Claims::new(claims);
    let header = Header::empty().with_token_type("JWT");
    let token = Ristretto256.token(&header, &claims, &Ristretto256SigningKey(profile.secret_key.clone()))?;
    Ok(token)
}
