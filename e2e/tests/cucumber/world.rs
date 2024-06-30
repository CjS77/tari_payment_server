use std::{
    collections::HashMap,
    sync::{mpsc::channel, Arc, Mutex},
};

use actix_web::dev::ServerHandle;
use chrono::Duration;
use cucumber::World;
use log::*;
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use tari_jwt::tari_crypto::ristretto::RistrettoSecretKey;
use tari_payment_engine::{
    db_types::SerializedTariAddress,
    events::{EventHandlers, EventHooks, EventType},
    test_utils::prepare_env::{create_database, random_db_path, run_migrations},
    traits::PaymentGatewayDatabase,
    SqliteDatabase,
};
use tari_payment_server::{
    config::{AuthConfig, ServerConfig},
    server::create_server_instance,
};
use tpg_common::Secret;

use crate::cucumber::setup::UserInfo;

#[derive(Debug, Clone, World)]
pub struct TPGWorld {
    pub config: ServerConfig,
    pub url: String,
    pub db: Option<SqliteDatabase>,
    pub super_admin: Option<UserInfo>,
    pub server_handle: Option<ServerHandle>,
    // The access token received from the server if a successful auth request was made
    pub access_token: Option<String>,
    pub logged_in: bool,
    pub response: Option<(StatusCode, String)>,
    pub wallets: HashMap<SerializedTariAddress, RistrettoSecretKey>,
    // Hashmap of order_id and whether the hook has been called.
    pub on_paid_hook_results: HashMap<String, bool>,
    pub last_event_type: Arc<Mutex<HashMap<&'static str, EventType>>>,
}

impl Default for TPGWorld {
    fn default() -> Self {
        let _ = env_logger::try_init().ok();
        let url = random_db_path();
        let config = ServerConfig {
            host: "127.0.0.1".into(),
            port: 20000 + rand::random::<u16>() % 10_000,
            shopify_api_key: String::default(),
            shopify_api_secret: Secret::default(),
            shopify_hmac_checks: false,
            database_url: url.clone(),
            auth: AuthConfig::default(),
            shopify_whitelist: None,
            use_x_forwarded_for: false,
            use_forwarded: false,
            unclaimed_order_timeout: Duration::seconds(2),
            unpaid_order_timeout: Duration::seconds(4),
        };
        Self {
            config,
            url,
            db: None,
            super_admin: None,
            server_handle: None,
            response: None,
            access_token: None,
            logged_in: false,
            wallets: HashMap::new(),
            on_paid_hook_results: HashMap::new(),
            last_event_type: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl TPGWorld {
    pub async fn start_database(&mut self) {
        let url = create_random_test_database().await;
        let db = SqliteDatabase::new_with_url(&url, 1).await.expect("Error creating connection to database");
        debug!("🌍️ Created database: {url}");
        self.db = Some(db);
    }

    pub fn database(&self) -> &SqliteDatabase {
        self.db.as_ref().expect("Database not started")
    }

    pub async fn start_server(&mut self) {
        let config = self.config.clone();
        if self.db.is_none() {
            panic!("🌍️ Database not started. Cannot start server.");
        }
        let db = self.db.as_ref().unwrap().clone();
        info!("🌍️ Starting server on {}:{} using DB {}", config.host, config.port, db.url());
        let last_event = Arc::clone(&self.last_event_type);
        let (tx, rx) = channel();
        tokio::spawn(async move {
            let mut hooks = EventHooks::default();
            let event = Arc::clone(&last_event);
            hooks.on_order_paid(move |ev| {
                info!("🌍️ Received order paid event: {ev:?}");
                if let Ok(mut events) = event.lock() {
                    events.insert("OrderPaid", EventType::OrderPaid(ev));
                }
                Box::pin(async {})
            });
            let event = Arc::clone(&last_event);
            hooks.on_order_annulled(move |ev| {
                info!("🌍️ Received order {} event: {ev:?}", ev.status.to_string().to_ascii_uppercase());
                if let Ok(mut le) = event.lock() {
                    le.insert("OrderAnnulled", EventType::OrderAnnulled(ev));
                }
                Box::pin(async {})
            });
            let event = Arc::clone(&last_event);
            hooks.on_order_modified(move |ev| {
                info!("🌍️ Received order modified event: {ev:?}");
                if let Ok(mut le) = event.lock() {
                    le.insert("OrderModified", EventType::OrderModified(ev));
                }
                Box::pin(async {})
            });
            let event = Arc::clone(&last_event);
            hooks.on_order_claimed(move |ev| {
                info!("🌍️ Received order claimed event: {ev:?}");
                if let Ok(mut le) = event.lock() {
                    le.insert("OrderClaimed", EventType::OrderClaimed(ev));
                }
                Box::pin(async {})
            });
            let event = Arc::clone(&last_event);
            hooks.on_new_order(move |ev| {
                info!("🌍️ Received new order event: {ev:?}");
                if let Ok(mut le) = event.lock() {
                    le.insert("NewOrder", EventType::NewOrder(ev));
                }
                Box::pin(async {})
            });
            let event = Arc::clone(&last_event);
            hooks.on_payment_received(move |ev| {
                info!("🌍️ Received payment event: {ev:?}");
                if let Ok(mut le) = event.lock() {
                    le.insert("PaymentReceived", EventType::PaymentReceived(ev));
                }
                Box::pin(async {})
            });
            let event = Arc::clone(&last_event);
            hooks.on_payment_confirmed(move |ev| {
                info!("🌍️ Received payment confirmation event: {ev:?}");
                if let Ok(mut le) = event.lock() {
                    le.insert("PaymentConfirmed", EventType::PaymentReceived(ev));
                }
                Box::pin(async {})
            });
            let handlers = EventHandlers::new(1, hooks);
            let producers = handlers.producers();
            let srv = create_server_instance(config, db, producers).expect("Error creating server instance");
            // Start the event handlers
            tokio::spawn(async move {
                handlers.start_handlers().await;
            });
            let _res = tx.send(srv.handle());
            match srv.await {
                Ok(_) => info!("🌍️ Server shut down"),
                Err(e) => warn!("🌍️ Server error: {e}"),
            }
        });
        let handle = rx.recv().unwrap();
        info!("🌍️ Server started");
        self.server_handle = Some(handle);
    }

    pub async fn get(&self, path: &str) -> (StatusCode, String) {
        self.request(Method::GET, path, |req| req).await
    }

    pub async fn request<F>(&self, method: Method, path: &str, req: F) -> (StatusCode, String)
    where F: FnOnce(RequestBuilder) -> RequestBuilder {
        let url = format!("http://{}:{}{path}", self.config.host, self.config.port);
        debug!("🌍️ Querying {url}");
        let client = Client::new();
        let request = client.request(method, url);
        let mut request = req(request);
        if let Some(token) = &self.access_token {
            debug!("🌍️ Adding auth token to request");
            request = request.header("tpg_access_token", token);
        }
        let res = request.send().await.expect("Error getting response");
        let code = res.status();
        let body = res.text().await.expect("Error parsing response body");
        (code, body)
    }

    pub fn last_event(&self, ev_type: &str) -> Option<EventType> {
        self.last_event_type.lock().expect("Another thread panicked while getting a lock").get(ev_type).cloned()
    }
}

pub async fn create_random_test_database() -> String {
    let path = random_db_path();
    create_database(&path).await;
    run_migrations(&path).await;
    path
}
