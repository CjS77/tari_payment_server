use std::{future::ready, net::SocketAddr, str::FromStr, time::Duration};

use actix_jwt_auth_middleware::use_jwt::UseJWTOnApp;
use actix_web::{
    dev::{Server, Service},
    http::KeepAlive,
    middleware::Logger,
    web,
    App,
    Error,
    HttpResponse,
    HttpServer,
};
use futures::{
    future::{ok, Either, LocalBoxFuture},
    FutureExt,
};
use log::{info, warn};
use tari_payment_engine::{AccountApi, AuthApi, OrderFlowApi, SqliteDatabase};

use crate::{
    auth::{build_tps_authority, TokenIssuer},
    config::ServerConfig,
    errors::{AuthError, ServerError, ServerError::AuthenticationError},
    routes::{
        health,
        AccountRoute,
        AuthRoute,
        CheckTokenRoute,
        MyAccountRoute,
        MyOrdersRoute,
        MyPaymentsRoute,
        OrderByIdRoute,
        OrdersRoute,
        OrdersSearchRoute,
        PaymentsRoute,
        ShopifyWebhookRoute,
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
            .service(MyPaymentsRoute::<SqliteDatabase>::new())
            .service(PaymentsRoute::<SqliteDatabase>::new())
            .service(OrdersSearchRoute::<SqliteDatabase>::new())
            .service(CheckTokenRoute::new());
        let use_x_forwarded_for = config.use_x_forwarded_for;
        let use_forwarded = config.use_forwarded;
        let shopify_whitelist = config.shopify_whitelist.clone();
        let shopify_scope = web::scope("/shopify")
            .wrap_fn(move |req, srv| {
                // Collect peer IP from x-forwarded-for, or forwarded headers _if_ `use_nnn` has been set to true
                // in the configuration. Otherwise, use the peer address from the connection info.
                let peer_addr = req.connection_info().peer_addr().map(|a| a.to_string());

                let peer_ip = req
                    .headers()
                    .get("X-Forwarded-For")
                    .and_then(|v| use_x_forwarded_for.then(|| v.to_str().ok()).flatten())
                    .or_else(|| {
                        req.headers().get("Forwarded").and_then(|v| use_forwarded.then(|| v.to_str().ok()).flatten())
                    })
                    .or_else(|| peer_addr.as_ref().map(|s| s.as_str()))
                    .and_then(|s| SocketAddr::from_str(s).ok());
                let whitelisted = match (peer_ip, &shopify_whitelist) {
                    (Some(ip), Some(whitelist)) => {
                        info!("Shopify webhook from {ip}");
                        whitelist.contains(&ip)
                    },
                    (_, None) => true,
                    (None, Some(_)) => {
                        warn!("No IP address found in shopify remote peer request, denying access.");
                        false
                    },
                };
                if whitelisted {
                    srv.call(req)
                } else {
                    ok(req.error_response(AuthenticationError(AuthError::ForbiddenPeer))).boxed_local()
                }
            })
            .service(ShopifyWebhookRoute::<SqliteDatabase>::new());
        app.use_jwt(authority.clone(), auth_scope)
            .service(health)
            .service(AuthRoute::<SqliteDatabase>::new())
            .service(shopify_scope)
    })
    .keep_alive(KeepAlive::Timeout(Duration::from_secs(600)))
    .bind((config.host.as_str(), config.port))?
    .run();
    Ok(srv)
}
