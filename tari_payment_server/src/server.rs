use crate::auth::{build_tps_authority, TokenIssuer};
use crate::config::ServerConfig;
use crate::errors::ServerError;
use crate::routes::{health, shopify_webhook, AuthRoute};

use actix_web::http::KeepAlive;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use std::time::Duration;
use tari_payment_engine::{AuthApi, OrderManagerApi, SqliteDatabase};

pub async fn run_server(config: ServerConfig) -> Result<(), ServerError> {
    let db = SqliteDatabase::new_with_url(&config.database_url)
        .await
        .map_err(|e| ServerError::InitializeError(e.to_string()))?;
    HttpServer::new(move || {
        let orders_api = OrderManagerApi::new(db.clone());
        let auth_api = AuthApi::new(db.clone());
        let jwt_signer = TokenIssuer::new(&config.auth);
        let _authority = build_tps_authority(config.auth.clone());
        App::new()
            .wrap(Logger::new("%t (%D ms) %s %a %{Host}i %U").log_target("webhook_listener"))
            .app_data(web::Data::new(orders_api))
            .app_data(web::Data::new(auth_api))
            .app_data(web::Data::new(jwt_signer))
            .service(health)
            .service(AuthRoute::<SqliteDatabase>::new())
            .service(web::scope("/shopify").service(shopify_webhook))
    })
    .keep_alive(KeepAlive::Timeout(Duration::from_secs(600)))
    .bind((config.host.as_str(), config.port))?
    .run()
    .await
    .map_err(|e| ServerError::Unspecified(e.to_string()))
}
