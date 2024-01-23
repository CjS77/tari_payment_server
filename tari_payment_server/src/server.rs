use crate::config::ServerConfig;
use crate::errors::ServerError;
use crate::routes::{health, shopify_webhook};
use actix_web::http::KeepAlive;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use std::time::Duration;

pub async fn run_server(config: ServerConfig) -> Result<(), ServerError> {
    //let pubsub = NewOrderNotifier::default().start();

    HttpServer::new(move || {
        App::new()
            //.app_data(Data::new(pubsub.clone()))
            .wrap(Logger::new("%t (%D ms) %s %a %{Host}i %U").log_target("webhook_listener"))
            .service(health)
            .service(web::scope("/shopify").service(shopify_webhook))
    })
    .keep_alive(KeepAlive::Timeout(Duration::from_secs(600)))
    .bind((config.host.as_str(), config.port))?
    .run()
    .await
    .map_err(|e| ServerError::Unspecified(e.to_string()))
}
