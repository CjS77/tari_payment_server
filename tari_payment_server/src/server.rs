use std::time::Duration;

use actix_jwt_auth_middleware::use_jwt::{UseJWTOnApp, UseJWTOnScope};
use actix_web::{dev::Server, http::KeepAlive, middleware::Logger, web, App, HttpServer};
use tari_payment_engine::{AccountApi, AuthApi, OrderManagerApi, SqliteDatabase};

use crate::{
    auth::{build_tps_authority, TokenIssuer},
    config::ServerConfig,
    errors::ServerError,
    routes::{health, shopify_webhook, AccountRoute, AuthRoute, MyAccountRoute},
};
use crate::middleware::AclMiddlewareFactory;
use crate::routes::{add_roles, remove_roles};

pub async fn run_server(config: ServerConfig) -> Result<(), ServerError> {
    let db = SqliteDatabase::new_with_url(&config.database_url, 25)
        .await
        .map_err(|e| ServerError::InitializeError(e.to_string()))?;
    let srv = create_server_instance(config, db)?;
    srv.await.map_err(|e| ServerError::Unspecified(e.to_string()))
}

pub fn create_server_instance(config: ServerConfig, db: SqliteDatabase) -> Result<Server, ServerError> {
    let srv = HttpServer::new(move || {
        let orders_api = OrderManagerApi::new(db.clone());
        let auth_api = AuthApi::new(db.clone());
        let jwt_signer = TokenIssuer::new(&config.auth);
        let authority = build_tps_authority(config.auth.clone());
        let accounts_api = AccountApi::new(db.clone());
        let app = App::new()
            .wrap(Logger::new("%t (%D ms) %s %a %{Host}i %U").log_target("webhook_listener"))
            .app_data(web::Data::new(orders_api))
            .app_data(web::Data::new(accounts_api))
            .app_data(web::Data::new(auth_api))
            .app_data(web::Data::new(jwt_signer))
            .service(health)
            .service(AuthRoute::<SqliteDatabase>::new())
            .service(
                web::scope("/shopify")
                    .service(shopify_webhook)
            );
        let admin_scope = web::scope("/admin")
            .service(add_roles)
            .service(remove_roles);
        let account_scope = web::scope("/account")
            .service(MyAccountRoute::<SqliteDatabase>::new())
            .service(AccountRoute::<SqliteDatabase>::new());
        let auth_scope = web::scope("")
            .service(admin_scope)
            .service(account_scope);
        app.use_jwt(authority.clone(), auth_scope)
    })
        .keep_alive(KeepAlive::Timeout(Duration::from_secs(600)))
        .bind((config.host.as_str(), config.port))?
        .run();
    Ok(srv)
}
