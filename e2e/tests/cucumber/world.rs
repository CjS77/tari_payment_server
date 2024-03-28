use std::sync::mpsc::channel;

use actix_web::dev::ServerHandle;
use cucumber::World;
use log::*;
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use tari_payment_engine::{
    test_utils::prepare_env::{create_database, random_db_path, run_migrations},
    PaymentGatewayDatabase,
    SqliteDatabase,
};
use tari_payment_server::{
    config::{AuthConfig, ServerConfig},
    server::create_server_instance,
};

#[derive(Debug, Clone, World)]
pub struct TPGWorld {
    pub config: ServerConfig,
    pub url: String,
    pub db: Option<SqliteDatabase>,
    pub server_handle: Option<ServerHandle>,
    pub response: Option<(StatusCode, String)>,
}

impl Default for TPGWorld {
    fn default() -> Self {
        let _ = env_logger::try_init().ok();
        let url = random_db_path();
        let config = ServerConfig {
            host: "127.0.0.1".into(),
            port: 20000 + rand::random::<u16>() % 10_000,
            shopify_api_key: String::default(),
            database_url: url.clone(),
            auth: AuthConfig::default(),
        };
        Self { config, url, db: None, server_handle: None, response: None }
    }
}

impl TPGWorld {
    pub fn set_auth_config(&mut self, auth: AuthConfig) {
        self.config.auth = auth;
    }

    pub fn refresh_auth_config(&mut self) -> AuthConfig {
        self.config.auth = AuthConfig::default();
        self.config.auth.clone()
    }

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
        let (tx, rx) = channel();
        tokio::spawn(async move {
            let srv = create_server_instance(config, db).expect("Error creating server instance");
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
        let url = format!("http://{}:{}/{path}", self.config.host, self.config.port);
        debug!("🌍️ Querying {url}");
        let res = reqwest::get(&url).await.expect("Error getting response");
        let code = res.status();
        let body = res.text().await.expect("Error parsing response body");
        (code, body)
    }

    pub async fn request<F>(&self, method: Method, path: &str, req: F) -> (StatusCode, String)
    where F: FnOnce(RequestBuilder) -> RequestBuilder {
        let url = format!("http://{}:{}/{path}", self.config.host, self.config.port);
        debug!("🌍️ Querying {url}");
        let client = Client::new();
        let request = client.request(method, url);
        let request = req(request);
        let res = request.send().await.expect("Error getting response");
        let code = res.status();
        let body = res.text().await.expect("Error parsing response body");
        (code, body)
    }
}

pub async fn create_random_test_database() -> String {
    let path = random_db_path();
    create_database(&path).await;
    run_migrations(&path).await;
    path
}
