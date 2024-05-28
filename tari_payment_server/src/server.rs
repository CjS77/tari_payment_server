use std::{net::IpAddr, time::Duration};

use actix_jwt_auth_middleware::use_jwt::UseJWTOnApp;
use actix_web::{
    dev::{Server, Service, ServiceRequest},
    http::KeepAlive,
    middleware::Logger,
    web,
    App,
    HttpServer,
};
use futures::{future::ok, FutureExt};
use log::*;
use tari_payment_engine::{
    events::{EventHandlers, EventHooks},
    AccountApi,
    AuthApi,
    OrderFlowApi,
    SqliteDatabase,
    WalletAuthApi,
};

use crate::{
    auth::{build_tps_authority, TokenIssuer},
    config::{ProxyConfig, ServerConfig},
    errors::{AuthError, ServerError, ServerError::AuthenticationError},
    helpers::get_remote_ip,
    routes::{
        health,
        AccountRoute,
        AuthRoute,
        CheckTokenRoute,
        IncomingPaymentNotificationRoute,
        MyAccountRoute,
        MyOrdersRoute,
        MyPaymentsRoute,
        OrderByIdRoute,
        OrdersRoute,
        OrdersSearchRoute,
        PaymentsRoute,
        ShopifyWebhookRoute,
        TxConfirmationNotificationRoute,
        UpdateRolesRoute,
    },
};

pub async fn run_server(config: ServerConfig) -> Result<(), ServerError> {
    let db = SqliteDatabase::new_with_url(&config.database_url, 25)
        .await
        .map_err(|e| ServerError::InitializeError(e.to_string()))?;
    let srv = create_server_instance(config, db, EventHooks::default())?;
    srv.await.map_err(|e| ServerError::Unspecified(e.to_string()))
}

pub fn create_server_instance(
    config: ServerConfig,
    db: SqliteDatabase,
    hooks: EventHooks,
) -> Result<Server, ServerError> {
    let proxy_config = ProxyConfig::from_config(&config);

    let handlers = EventHandlers::new(128, hooks);
    let producers = handlers.producers();

    let srv = HttpServer::new(move || {
        let orders_api = OrderFlowApi::new(db.clone(), producers.clone());
        let auth_api = AuthApi::new(db.clone());
        let jwt_signer = TokenIssuer::new(&config.auth);
        let authority = build_tps_authority(config.auth.clone());
        let accounts_api = AccountApi::new(db.clone());
        let wallet_auth = WalletAuthApi::new(db.clone());

        let mut app = App::new()
            .wrap(Logger::new("%t (%D ms) %s %a %{Host}i %U").log_target("tps::access_log"))
            .app_data(web::Data::new(orders_api))
            .app_data(web::Data::new(accounts_api))
            .app_data(web::Data::new(auth_api))
            .app_data(web::Data::new(jwt_signer))
            .app_data(web::Data::new(wallet_auth))
            .app_data(web::Data::new(proxy_config));
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
                let whitelisted = is_whitelisted(use_x_forwarded_for, use_forwarded, &shopify_whitelist, &req);
                if whitelisted {
                    srv.call(req)
                } else {
                    ok(req.error_response(AuthenticationError(AuthError::ForbiddenPeer))).boxed_local()
                }
            })
            .service(ShopifyWebhookRoute::<SqliteDatabase>::new());
        let wallet_scope = web::scope("/wallet")
            .service(IncomingPaymentNotificationRoute::<SqliteDatabase, SqliteDatabase>::new())
            .service(TxConfirmationNotificationRoute::<SqliteDatabase, SqliteDatabase>::new());
        app = app.service(wallet_scope);
        app.use_jwt(authority.clone(), auth_scope)
            .service(health)
            .service(AuthRoute::<SqliteDatabase>::new())
            .service(shopify_scope)
    })
    .keep_alive(KeepAlive::Timeout(Duration::from_secs(600)))
    .bind((config.host.as_str(), config.port))?
    .run();

    // Start the event handlers
    tokio::spawn(async move {
        handlers.start_handlers().await;
    });
    Ok(srv)
}

fn is_whitelisted(
    use_x_forwarded_for: bool,
    use_forwarded: bool,
    shopify_whitelist: &Option<Vec<IpAddr>>,
    req: &ServiceRequest,
) -> bool {
    let peer_ip = get_remote_ip(req.request(), use_x_forwarded_for, use_forwarded);
    match (peer_ip, &shopify_whitelist) {
        (Some(ip), Some(whitelist)) => {
            info!("Shopify webhook from {ip}");
            whitelist.contains(&ip)
        },
        (_, None) => true,
        (None, Some(_)) => {
            warn!("No IP address found in shopify remote peer request, denying access.");
            false
        },
    }
}
