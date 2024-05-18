use std::time::Duration;

use actix_jwt_auth_middleware::use_jwt::UseJWTOnApp;
use actix_web::{dev::Server, http::KeepAlive, middleware::Logger, web, App, HttpServer};
use tari_payment_engine::{AccountApi, AuthApi, OrderFlowApi, SqliteDatabase};

use crate::{
    auth::{build_tps_authority, TokenIssuer},
    config::ServerConfig,
    errors::ServerError,
    routes::{
        health,
        shopify_webhook,
        AccountRoute,
        AuthRoute,
        CheckTokenRoute,
        MyAccountRoute,
        MyOrdersRoute,
        OrderByIdRoute,
        OrdersRoute,
        UpdateRolesRoute,
    },
};

pub async fn run_server(config: ServerConfig) -> Result<(), ServerError> {
    let db = SqliteDatabase::new_with_url(&config.database_url, 25)
        .await
        .map_err(|e| ServerError::InitializeError(e.to_string()))?;
    let srv = create_server_instance(config, db)?;
    srv.await.map_err(|e| ServerError::Unspecified(e.to_string()))
}

pub fn create_server_instance(config: ServerConfig, db: SqliteDatabase) -> Result<Server, ServerError> {
    let srv = HttpServer::new(move || {
        let orders_api = OrderFlowApi::new(db.clone());
        let auth_api = AuthApi::new(db.clone());
        let jwt_signer = TokenIssuer::new(&config.auth);
        let authority = build_tps_authority(config.auth.clone());
        let accounts_api = AccountApi::new(db.clone());
        let app = App::new()
            .wrap(Logger::new("%t (%D ms) %s %a %{Host}i %U").log_target("tps::access_log"))
            .app_data(web::Data::new(orders_api))
            .app_data(web::Data::new(accounts_api))
            .app_data(web::Data::new(auth_api))
            .app_data(web::Data::new(jwt_signer));
        // Routes that require authentication
        let auth_scope = web::scope("/api")
            .service(UpdateRolesRoute::<SqliteDatabase>::new())
            .service(MyAccountRoute::<SqliteDatabase>::new())
            .service(AccountRoute::<SqliteDatabase>::new())
            .service(MyOrdersRoute::<SqliteDatabase>::new())
            .service(OrdersRoute::<SqliteDatabase>::new())
            .service(OrderByIdRoute::<SqliteDatabase>::new())
            .service(CheckTokenRoute::new());
        app.use_jwt(authority.clone(), auth_scope)
            .service(health)
            .service(AuthRoute::<SqliteDatabase>::new())
            .service(web::scope("/shopify").service(shopify_webhook))
    })
    .keep_alive(KeepAlive::Timeout(Duration::from_secs(600)))
    .bind((config.host.as_str(), config.port))?
    .run();
    Ok(srv)
}
