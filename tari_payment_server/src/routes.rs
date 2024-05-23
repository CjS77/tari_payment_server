//! Request handler definitions
//!
//! Define each route and it handler here.
//! Handlers that are more than a line or two MUST go into a separate module. Keep this module neat and tidy 🙏
//!
//! A note about performance:
//! Since each worker thread processes its requests sequentially, handlers which block the current thread will cause the
//! current worker to stop processing new requests:
//! ```nocompile
//!     fn my_handler() -> impl Responder {
//!         std::thread::sleep(Duration::from_secs(5)); // <-- Bad practice! Will cause the current worker thread to
//! hang!
//!     }
//! ```
//! For this reason, any long, non-cpu-bound operation (e.g. I/O, database operations, etc.) should be expressed as
//! futures or asynchronous functions. Async handlers get executed concurrently by worker threads and thus don’t block
//! execution:
//!
//! ```nocompile
//!     async fn my_handler() -> impl Responder {
//!         tokio::time::sleep(Duration::from_secs(5)).await; // <-- Ok. Worker thread will handle other requests here
//!     }
//! ```
use std::{marker::PhantomData, str::FromStr};

use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use log::*;
use paste::paste;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::{NewOrder, OrderId, Role, SerializedTariAddress},
    order_objects::OrderQueryFilter,
    traits::{AccountManagement, AuthManagement, PaymentGatewayDatabase, PaymentGatewayError, WalletAuth},
    AccountApi,
    AuthApi,
    OrderFlowApi,
    WalletAuthApi,
};

use crate::{
    auth::{check_login_token_signature, JwtClaims, TokenIssuer},
    config::ProxyConfig,
    data_objects::{JsonResponse, PaymentNotification, RoleUpdateRequest},
    errors::{OrderConversionError, ServerError},
    helpers::get_remote_ip,
    shopify_order::ShopifyOrder,
};

// Web-actix cannot handle generics in handlers, so it's implemented manually using the `route!` macro
macro_rules! route {
    ($name:ident => $method:ident $path:literal requires [$($roles:ty),*]) => {
        paste! { pub struct [<$name:camel Route>];}
        paste! {
                impl [<$name:camel Route>] {
                #[allow(clippy::new_without_default)]
                pub fn new() -> Self { Self }
            }
        }
        paste! {
            impl actix_web::dev::HttpServiceFactory for [<$name:camel Route>] {
                fn register(self, config: &mut actix_web::dev::AppService) {
                    let res = actix_web::Resource::new($path)
                        .name(stringify!($name))
                        .guard(actix_web::guard::$method())
                        .to($name)
                        .wrap(crate::middleware::AclMiddlewareFactory::new(&[$($roles),+]));
                    actix_web::dev::HttpServiceFactory::register(res, config);
                }
            }
        }
    };

    ($name:ident => $method:ident $path:literal impl $($bounds:ty),+) => {
        paste! { pub struct [<$name:camel Route>]< $( [< T $bounds:camel> ],)+ >( $( PhantomData<fn() -> [< T $bounds:camel> ] >,)+ );}
        paste! { impl< $( [< T $bounds:camel> ],)+ > [<$name:camel Route>]< $( [< T $bounds:camel> ],)+ > {
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self($( PhantomData::<fn() -> [< T $bounds:camel> ] >,)+)
            }
        }}
        paste! { impl<$( [< T $bounds:camel >] , )+> actix_web::dev::HttpServiceFactory for [<$name:camel Route>]<$([<T $bounds:camel>],)+>
        where
            $([<T $bounds:camel>]: $bounds + 'static,)+
        {
            fn register(self, config: &mut actix_web::dev::AppService) {
                let res = actix_web::Resource::new($path)
                    .name(stringify!($name))
                    .guard(actix_web::guard::$method())
                    .to($name::< $( [< T $bounds:camel >], )+>);
                actix_web::dev::HttpServiceFactory::register(res, config);
            }
        }}
    };

    ($name:ident => $method:ident $path:literal impl $($bounds:ty),+ where requires [$($roles:ty),*])  => {
        paste! { pub struct [<$name:camel Route>]<A>(PhantomData<fn() -> A>);}
        paste! { impl<A> [<$name:camel Route>]<A> {
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self(PhantomData::<fn() -> A>)
            }
        }}
        paste! { impl<A> actix_web::dev::HttpServiceFactory for [<$name:camel Route>]<A>
        where
            A: $($bounds)++ 'static,
        {
            fn register(self, config: &mut actix_web::dev::AppService) {
                let res = actix_web::Resource::new($path)
                    .name(stringify!($name))
                    .guard(actix_web::guard::$method())
                    .to($name::<A>)
                    .wrap(crate::middleware::AclMiddlewareFactory::new(&[$($roles),+]));
                actix_web::dev::HttpServiceFactory::register(res, config);
            }
        }}
    };
}

// ----------------------------------------------   Health  ----------------------------------------------------
#[get("/health")]
pub async fn health() -> impl Responder {
    trace!("💻️ Received health check request");
    HttpResponse::Ok().body("👍️\n")
}

//----------------------------------------------   Auth  ----------------------------------------------------
route!(auth => Post "/auth" impl AuthManagement);
/// Route handler for the auth endpoint
///
/// This route is used to authenticate a user and issue a JWT token.
///
/// Users must supply a login token in the `tpg_auth_token` header.
/// This token is signed by the user('s wallet, typically) and is a JWT with the following fields (See [`LoginToken`]):
/// * `address` - The address of the user's wallet. This is the same as the pubkey with an additional checksum/network
///   byte.
/// * `nonce` - A unique number that must increase on every call (not necessarily by 1 - a unix time epoch can be used,
///   for example).
/// * `desired_roles` - A list of roles that the user wants to have. This is used to request additional permissions.
///
/// If successful, the server will issue a JWT token that can be used to authenticate future requests.
/// The JWT is valid for a relatively short period and will NOT refresh.
//#[post("/auth")]
pub async fn auth<A>(
    req: HttpRequest,
    api: web::Data<AuthApi<A>>,
    signer: web::Data<TokenIssuer>,
) -> Result<HttpResponse, ServerError>
where
    A: AuthManagement,
{
    trace!("💻️ Received auth request");
    let payload = req.headers().get("tpg_auth_token").ok_or(ServerError::CouldNotDeserializeAuthToken)?;
    let login_token = payload.to_str().map_err(|e| {
        debug!("💻️ Could not read auth token. {e}");
        ServerError::CouldNotDeserializeAuthToken
    })?;
    let token = check_login_token_signature(login_token)?;
    debug!("💻️ Login token was validated for {token:?}");
    api.upsert_nonce_for_address(&token.address, token.nonce).await?;
    trace!("💻️ Confirming auth request is valid for roles for {}", token.address);
    api.check_address_has_roles(&token.address, &token.desired_roles).await.map_err(|e| {
        debug!("💻️ User cannot be authenticated for requested roles. {e}");
        ServerError::InsufficientPermissions(e.to_string())
    })?;
    let access_token = signer.issue_token(token, None)?;
    trace!("💻️ Issued access token");
    Ok(HttpResponse::Ok().content_type("application/json").body(access_token))
}

//----------------------------------------------   Account  ----------------------------------------------------

route!(my_account => Get "/account" impl AccountManagement);
/// Route handler for the account endpoint
///
/// This route is used to fetch account information for a given address. The address that is queried is the one that
/// is associated with the JWT token that is supplied in the `tpg_access_token` header.
///
/// To access other accounts, the user must have the `ReadAll` role and can use the `/account/{address}` endpoint.
//#[get("/account/")]
pub async fn my_account<B: AccountManagement>(
    claims: JwtClaims,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET my_account for {}", claims.address);
    get_account(&claims.address, api.as_ref()).await
}

route!(account => Get "/account/{address}" impl AccountManagement where requires [Role::ReadAll]);
/// Route handler for the account/{address} endpoint
///
/// This route is used to fetch account information for the address supplied in the query path
///
/// To access other accounts, the user must have the `ReadAll` role and can use the `/account/{address}` endpoint.
/// Otherwise, the user can only access their own account. It is usually more convenient to use the `/account` endpoint
/// for this purpose.
//#[get("/account/{address}")]
pub async fn account<B: AccountManagement>(
    path: web::Path<SerializedTariAddress>,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let address = path.into_inner().to_address();
    debug!("💻️ GET account for {address}");
    get_account(&address, api.as_ref()).await
}

pub async fn get_account<B: AccountManagement>(
    address: &TariAddress,
    api: &AccountApi<B>,
) -> Result<HttpResponse, ServerError> {
    let account = api.account_by_address(address).await.map_err(|e| {
        debug!("💻️ Could not fetch account. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    match account {
        Some(acc) => Ok(HttpResponse::Ok().json(acc)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

//----------------------------------------------   Orders  ----------------------------------------------------

route!(my_orders => Get "/orders" impl AccountManagement);
/// Route handler for the orders endpoint
///
/// Authenticated users can fetch their own orders using this endpoint. The Tari address for the account is extracted
/// from the JWT token supplied in the `tpg_access_token` header.
///
/// Admin users (ReadAll and SuperAdmin roles) can use the `/orders/{address}` endpoint to fetch orders for any account.
pub async fn my_orders<B: AccountManagement>(
    claims: JwtClaims,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET my_orders for {}", claims.address);
    get_orders(&claims.address, api.as_ref()).await
}

route!(orders_search => Get "/search/orders" impl AccountManagement where requires [Role::ReadAll]);
pub async fn orders_search<B: AccountManagement>(
    query: web::Query<OrderQueryFilter>,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET orders search for [{query}]");
    let query = query.into_inner();
    let orders = api.search_orders(query).await.map_err(|e| {
        debug!("💻️ Could not fetch orders. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    Ok(HttpResponse::Ok().json(orders))
}

route!(orders => Get "/orders/{address}" impl AccountManagement where requires [Role::ReadAll]);
/// Route handler for the orders/{address} endpoint
///
/// Admin users (ReadAll and SuperAdmin roles) can fetch orders for any account using this endpoint.
pub async fn orders<B: AccountManagement>(
    path: web::Path<SerializedTariAddress>,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let address = path.into_inner().to_address();
    debug!("💻️ GET orders for {address}");
    get_orders(&address, api.as_ref()).await
}

route!(order_by_id => Get "/order/id/{order_id}" impl AccountManagement where requires [Role::User]);
/// User `/order/id/{order_id}` to fetch a specific order by its order_id.
///
/// Authenticated users can fetch their own orders using this endpoint. The Tari address for the account is extracted
/// from the JWT token supplied in the `tpg_access_token` header. Any other order ids supplied return null, whether they
/// exist or not.
///
/// Admin users (ReadAll and SuperAdmin roles) will be able to retrieve any order by its order_id.
pub async fn order_by_id<B: AccountManagement>(
    claims: JwtClaims,
    path: web::Path<OrderId>,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let order_id = path.into_inner();
    debug!("💻️ GET order by id for {order_id}");
    let address = claims.address;

    // There's no particular ACL on this route, so check that the order belongs to the user,
    // OR they have the `ReadAll`/`SuperAdmin` role
    let is_admin = claims.roles.contains(&Role::ReadAll) || claims.roles.contains(&Role::SuperAdmin);
    if is_admin {
        let order = api.as_ref().fetch_order_by_order_id(&order_id).await.map_err(|e| {
            debug!("💻️ Could not fetch order. {e}");
            ServerError::BackendError(e.to_string())
        })?;
        return Ok(HttpResponse::Ok().json(order));
    }
    // We need to do some extra checks to make sure the user may see this order
    let orders = api.orders_for_address(&address).await.map_err(|e| {
        debug!("💻️ Could not fetch order. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    let result = orders.and_then(|orders| orders.orders.into_iter().find(|o| o.order_id == order_id));
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_orders<B: AccountManagement>(
    address: &TariAddress,
    api: &AccountApi<B>,
) -> Result<HttpResponse, ServerError> {
    match api.orders_for_address(address).await {
        Ok(Some(orders)) => Ok(HttpResponse::Ok().json(orders)),
        Ok(None) => Ok(HttpResponse::NotFound().finish()),
        Err(e) => {
            debug!("💻️ Could not fetch orders. {e}");
            Err(ServerError::BackendError(e.to_string()))
        },
    }
}

//----------------------------------------------   Payments  ----------------------------------------------------

route!(my_payments => Get "/payments" impl AccountManagement);
/// Route handler for the payments endpoint
///
/// Authenticated users can fetch their own payments using this endpoint. The Tari address for the account is extracted
/// from the JWT token supplied in the `tpg_access_token` header.
///
/// Admin users (ReadAll and SuperAdmin roles) can use the `/payments/{address}` endpoint to fetch payments for any
/// wallet address.
pub async fn my_payments<B: AccountManagement>(
    claims: JwtClaims,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET my_payments for {}", claims.address);
    get_payments(&claims.address, api.as_ref()).await
}

route!(payments => Get "/payments/{address}" impl AccountManagement where requires [Role::ReadAll]);
/// Route handler for the payments/{address} endpoint
///
/// Admin users (ReadAll and SuperAdmin roles) can fetch payments for any account using this endpoint. Other users
/// will receive a 401 Unauthorized response.
pub async fn payments<B: AccountManagement>(
    path: web::Path<SerializedTariAddress>,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let address = path.into_inner().to_address();
    debug!("💻️ GET orders for {address}");
    get_payments(&address, api.as_ref()).await
}

async fn get_payments<B>(address: &TariAddress, api: &AccountApi<B>) -> Result<HttpResponse, ServerError>
where B: AccountManagement {
    match api.payments_for_address(address).await {
        Ok(payments) => Ok(HttpResponse::Ok().json(payments)),
        Err(e) => {
            debug!("💻️ Could not fetch payments. {e}");
            Err(ServerError::BackendError(e.to_string()))
        },
    }
}

//----------------------------------------------   Checkout  ----------------------------------------------------

route!(shopify_webhook => Post "webhook/checkout_create" impl PaymentGatewayDatabase);
pub async fn shopify_webhook<B: PaymentGatewayDatabase>(
    req: HttpRequest,
    body: web::Json<ShopifyOrder>,
    api: web::Data<OrderFlowApi<B>>,
) -> HttpResponse {
    trace!("💻️ Received webhook request: {}", req.uri());
    let order = body.into_inner();
    // Webhook responses must always be in 200 range, otherwise Shopify will retry
    let result = match NewOrder::try_from(order) {
        Err(OrderConversionError::FormatError(s)) => {
            warn!("💻️ Could not convert order. {s}");
            JsonResponse::failure(s)
        },
        Err(OrderConversionError::InvalidMemoSignature(e)) => {
            warn!("💻️ Could not verify memo signature. {e}");
            JsonResponse::failure(e)
        },
        Err(OrderConversionError::UnsupportedCurrency(cur)) => {
            info!("💻️ Unsupported currency in incoming order. {cur}");
            JsonResponse::failure(format!("Unsupported currency: {cur}"))
        },
        Ok(new_order) => match api.process_new_order(new_order.clone()).await {
            Ok(orders) => {
                info!("💻️ Order {} processed successfully.", new_order.order_id);
                let ids = orders.iter().map(|o| o.order_id.as_str()).collect::<Vec<_>>().join(", ");
                info!("💻️ {} orders were paid. {}", orders.len(), ids);
                JsonResponse::success("Order processed successfully.")
            },
            Err(PaymentGatewayError::DatabaseError(e)) => {
                warn!("💻️ Could not process order {}. {e}", new_order.order_id);
                debug!("💻️ Failed order: {new_order}");
                JsonResponse::failure(e)
            },
            Err(PaymentGatewayError::OrderAlreadyExists(id)) => {
                info!("💻️ Order {} already exists with id {id}.", new_order.order_id);
                JsonResponse::success("Order already exists.")
            },
            Err(e) => {
                warn!("💻️ Unexpected error while handling incoming order notification. {e}");
                JsonResponse::failure("Unexpected error handling order.")
            },
        },
    };
    HttpResponse::Ok().json(result)
}

//------------------------------------------   Incoming payments  ---------------------------------------------
route!(incoming_payment_notification => Post "/incoming_payment" impl PaymentGatewayDatabase, WalletAuth );
pub async fn incoming_payment_notification<BOrder, BAuth>(
    req: HttpRequest,
    config: web::Data<ProxyConfig>,
    auth_api: web::Data<WalletAuthApi<BAuth>>,
    order_api: web::Data<OrderFlowApi<BOrder>>,
    body: web::Json<PaymentNotification>,
) -> HttpResponse
where
    BAuth: WalletAuth,
    BOrder: PaymentGatewayDatabase,
{
    trace!("💻️ Received incoming payment notification");
    let PaymentNotification { payment, auth } = body.into_inner();
    let use_x_forwarded_for = config.use_x_forwarded_for;
    let use_forwarded = config.use_forwarded;
    trace!("💻️ Extracting remote IP address. {req:?}. {:?}", req.connection_info());
    let Some(peer_addr) = get_remote_ip(&req, use_x_forwarded_for, use_forwarded) else {
        warn!("💻️ Could not determine remote IP address for a wallet payment notification. The request is rejected");
        return HttpResponse::Unauthorized().finish();
    };
    // Log the payment
    info!("💻️ New payment notification received from IP {peer_addr}.");
    info!("💻️ Payment: {}", serde_json::to_string(&payment).unwrap_or_else(|e| format!("{e}")));
    info!("💻️ Auth: {}", serde_json::to_string(&auth).unwrap_or_else(|e| format!("{e}")));
    trace!("💻️ Verifying wallet signature");
    if !auth.is_valid(&payment) {
        warn!("💻️ Invalid wallet signature received from {peer_addr}. The request is rejected.");
        return HttpResponse::Unauthorized().finish();
    }
    let auth_api = auth_api.as_ref();
    if let Err(e) = auth_api.authenticate_wallet(auth, &peer_addr, &payment).await {
        warn!("💻️ Unauthorized wallet signature received from {peer_addr}. Reason: {e}. The request is rejected.");
        return HttpResponse::Unauthorized().finish();
    }
    // -- from here on, we trust that the notification is legitimate.
    let result = match order_api.process_new_payment(payment).await {
        Ok(orders) => {
            let ids = orders.iter().map(|o| o.order_id.as_str()).collect::<Vec<_>>().join(", ");
            let msg = format!("{} orders were paid. {}", orders.len(), ids);
            info!("💻️ {msg}");
            JsonResponse::success(msg)
        },
        Err(PaymentGatewayError::DatabaseError(e)) => {
            warn!("💻️ Could not process payment. {e}");
            JsonResponse::failure(e)
        },
        Err(PaymentGatewayError::PaymentAlreadyExists(id)) => {
            info!("💻️ Payment already exists with id {id}.");
            JsonResponse::success("Payment already exists.")
        },
        Err(e) => {
            warn!("💻️ Unexpected error handling incoming payment notification. {e}");
            JsonResponse::failure("Unexpected error handling payment.")
        },
    };
    HttpResponse::Ok().json(result)
}

//----------------------------------------------   Roles  ----------------------------------------------------
route!(update_roles => Post "/roles" impl AuthManagement where requires [Role::SuperAdmin]);
pub async fn update_roles<B: AuthManagement>(
    api: web::Data<AuthApi<B>>,
    body: web::Json<Vec<RoleUpdateRequest>>,
) -> Result<HttpResponse, ServerError> {
    for acl_request in body.into_inner() {
        let address = acl_request.address;
        let address = TariAddress::from_str(&address).map_err(|e| {
            debug!("💻️ Could not parse address. {e}");
            ServerError::InvalidRequestPath(e.to_string())
        })?;
        debug!("💻️ POST update roles for {address}");
        api.assign_roles(&address, &acl_request.apply).await?;
        api.remove_roles(&address, &acl_request.revoke).await?;
    }
    Ok(HttpResponse::Ok().finish())
}

//----------------------------------------------  Check Token  ----------------------------------------------------
route!(check_token => Get "/check_token" requires [Role::User]);
pub async fn check_token(claims: JwtClaims) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET check_token for {}", claims.address);
    Ok(HttpResponse::Ok().body("Token is valid."))
}
