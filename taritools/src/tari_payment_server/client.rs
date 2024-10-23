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
    db_types::{
        AddressBalance,
        CreditNote,
        CustomerOrders,
        LoginToken,
        Order,
        OrderId,
        Payment,
        Role,
        SerializedTariAddress,
    },
    helpers::MemoSignature,
    order_objects::{ClaimedOrder, OrderChanged, OrderResult},
    tpe_api::{
        account_objects::{AddressHistory, CustomerHistory},
        payment_objects::PaymentsResult,
    },
    traits::{NewWalletInfo, OrderMovedResult, WalletInfo},
};
use tari_payment_server::data_objects::{
    ExchangeRateResult,
    ExchangeRateUpdate,
    JsonResponse,
    ModifyOrderParams,
    MoveOrderParams,
    PaymentNotification,
    TransactionConfirmationNotification,
    UpdateMemoParams,
};
use tpg_common::MicroTari;
use url::Url;

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
        self.profile.server.as_str()
    }

    pub fn profile_name(&self) -> &str {
        &self.profile.name
    }

    pub fn roles(&self) -> Vec<Role> {
        self.profile.roles.clone()
    }

    pub fn url(&self, path: &str) -> Result<Url> {
        self.profile.server.join(path).map_err(|e| anyhow!("Failed to join URL: {}", e))
    }

    pub async fn authenticate(&mut self) -> Result<()> {
        println!("Authenticating with payment server");
        let token = generate_auth_token(&self.profile)?;
        let url = self.url("/auth")?;
        println!("url: {}", url);
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
        let url = self.url("/health")?;
        let res = self.client.get(url).send().await?;
        let response = res.text().await?;
        Ok(response)
    }

    /// Retrieve the list of Tari addresses that can be used to send payments to the payment server.
    ///
    /// This method does not require authentication.
    pub async fn payment_addresses(&self) -> Result<Vec<TariAddress>> {
        let url = self.url("/wallet/send_to")?;
        let res = self.client.get(url).send().await?;
        let addresses = res.json::<Vec<SerializedTariAddress>>().await?;
        let addresses = addresses.into_iter().map(|a| a.to_address()).collect();
        Ok(addresses)
    }

    pub async fn add_authorized_wallet(&self, wallet: &NewWalletInfo) -> Result<()> {
        let url = self.url("/api/wallets")?;
        let res =
            self.client.post(url).header("tpg_access_token", self.access_token.clone()).json(wallet).send().await?;
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error adding wallet: {msg}"));
        }
        Ok(())
    }

    pub async fn remove_authorized_wallet(&self, address: &TariAddress) -> Result<()> {
        let url = self.url(&format!("/api/wallets/{}", address.to_base58()))?;
        let res = self.client.delete(url).header("tpg_access_token", self.access_token.clone()).send().await?;
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error removing wallet: {msg}"));
        }
        Ok(())
    }

    pub async fn my_balance(&self) -> Result<AddressBalance> {
        let url = self.url("/api/balance")?;
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

    pub async fn my_account(&self) -> Result<()> {
        let url = self.url("/api/account")?;
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

    pub async fn authorized_wallets(&self) -> Result<Vec<WalletInfo>> {
        self.auth_get_request("/api/wallets").await
    }

    pub async fn claim_order(&self, signature: &MemoSignature) -> Result<ClaimedOrder> {
        let url = self.url("/order/claim")?;
        let res = self.client.post(url).json(&signature).send().await?;
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error setting exchange rates: {msg}"));
        }
        let claimed_order = res.json().await?;
        Ok(claimed_order)
    }

    pub async fn set_exchange_rate(&self, currency: &str, price_in_tari: MicroTari) -> Result<()> {
        let url = self.url("/api/exchange_rate")?;
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
        let url = self.url("/wallet/incoming_payment")?;
        let res = self.client.post(url).json(&notification).send().await?;
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error sending payment notification: {msg}"));
        }
        Ok(())
    }

    /// Send a payment confirmation to the payment server.
    pub async fn payment_confirmation(&self, confirmation: TransactionConfirmationNotification) -> Result<()> {
        let url = self.url("/wallet/tx_confirmation")?;
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

    pub async fn my_unfulfilled_orders(&self) -> Result<OrderResult> {
        self.auth_get_request("/api/unfulfilled_orders").await
    }

    pub async fn my_payments(&self) -> Result<PaymentsResult> {
        self.auth_get_request("/api/payments").await
    }

    /// Returns the Account History (or full account) for the authenticated address (anchor address).
    ///
    /// **Note**: This could return orders and payments that are not present in [`my_orders`], or [`my_payments']
    /// respectively, since these methods only provide data directly linked to the anchor address, whereas `my_history`
    /// does a reverse link search to find _all_ addresses attached to the account before querying for orders and
    /// payments.
    pub async fn my_history(&self) -> Result<AddressHistory> {
        self.auth_get_request("/api/history").await
    }

    pub async fn history_for_address(&self, address: &TariAddress) -> Result<AddressHistory> {
        self.auth_get_request(&format!("/api/history/address/{}", address.to_base58())).await
    }

    pub async fn balance_for_address(&self, address: &TariAddress) -> Result<AddressBalance> {
        self.auth_get_request(&format!("/api/balance/{}", address.to_base58())).await
    }

    pub async fn history_for_id(&self, cust_id: &str) -> Result<CustomerHistory> {
        self.auth_get_request(&format!("/api/history/customer/{cust_id}")).await
    }

    async fn auth_get_request<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.url(path)?;
        let res = self.client.get(url).header("tpg_access_token", self.access_token.clone()).send().await?;
        match res.status() {
            StatusCode::OK => Ok({
                res.json().await?
                // let body: Value = res.json().await?;
                // println!("body: {:?}", body);
                // serde_json::from_value(body).unwrap_or_else(|e| format!("Could not represent response as JSON. {e}"))
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
        self.auth_get_request(&format!("/api/orders/{}", address.to_base58())).await
    }

    pub async fn rescan_open_orders(&self) -> Result<Vec<JsonResponse>> {
        let url = self.url("/api/rescan_open_orders")?;
        let res = self.client.post(url).header("tpg_access_token", self.access_token.clone()).send().await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            code => {
                let msg = res.text().await?;
                Err(anyhow!("Error rescanning open orders: {code}, {msg}."))
            },
        }
    }

    pub async fn order_by_id(&self, order_id: &OrderId) -> Result<Option<Order>> {
        let id = urlencoding::encode(order_id.as_str());
        self.auth_get_request(&format!("/api/order/id/{id}")).await
    }

    pub async fn cancel_order(&self, params: &ModifyOrderParams) -> Result<Order> {
        let url = self.url("/api/cancel")?;
        let res =
            self.client.post(url).header("tpg_access_token", self.access_token.clone()).json(params).send().await?;
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error cancelling order: {msg}"));
        }
        let order = res.json().await?;
        Ok(order)
    }

    pub async fn fulfil_order(&self, params: &ModifyOrderParams) -> Result<Order> {
        let url = self.url("/api/fulfill")?;
        let res =
            self.client.post(url).header("tpg_access_token", self.access_token.clone()).json(params).send().await?;
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error fulfilling order: {msg}"));
        }
        let order = res.json().await?;
        Ok(order)
    }

    pub async fn reset_order(&self, order_id: &OrderId) -> Result<Order> {
        let url = self.url(&format!("/api/reset_order/{order_id}"))?;
        let res = self.client.patch(url).header("tpg_access_token", self.access_token.clone()).send().await?;
        let code = res.status();
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error {code}. Could not reset order. {msg}"));
        }
        let changes: OrderChanged = res.json().await?;
        Ok(changes.new_order)
    }

    pub async fn payments_for_address(&self, address: TariAddress) -> Result<PaymentsResult> {
        self.auth_get_request(&format!("/api/payments/{}", address.to_base58())).await
    }

    pub async fn payments_for_order(&self, order_id: &OrderId) -> Result<Vec<Payment>> {
        self.auth_get_request(&format!("/api/payments-for-order/{order_id}")).await
    }

    pub async fn issue_credit(&self, customer_id: &str, amount: MicroTari, reason: String) -> Result<Vec<Order>> {
        let url = self.url("/api/credit")?;
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

    pub async fn edit_memo(&self, params: &UpdateMemoParams) -> Result<Order> {
        let url = self.url("/api/order_memo")?;
        let res =
            self.client.patch(url).header("tpg_access_token", self.access_token.clone()).json(params).send().await?;
        let code = res.status();
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error {code}. Could not edit memo. {msg}"));
        }
        let order: Order = res.json().await?;
        Ok(order)
    }

    pub async fn reassign_order(&self, params: &MoveOrderParams) -> Result<OrderMovedResult> {
        let url = self.url("/api/reassign_order")?;
        let res =
            self.client.patch(url).header("tpg_access_token", self.access_token.clone()).json(params).send().await?;
        let code = res.status();
        if !res.status().is_success() {
            let msg = res.text().await?;
            return Err(anyhow!("Error {code}. Could not reassign order. {msg}"));
        }
        let result: OrderMovedResult = res.json().await?;
        Ok(result)
    }

    pub async fn creditors(&self) -> Result<Vec<CustomerOrders>> {
        self.auth_get_request("/api/creditors").await
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
