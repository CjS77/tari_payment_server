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
use std::{ops::Deref, str::FromStr};

use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use log::*;
use serde_json::json;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{
    db_types::{CreditNote, Order, OrderId, OrderStatusType, Role, SerializedTariAddress},
    helpers::MemoSignature,
    order_objects::{OrderQueryFilter, OrderResult},
    tpe_api::{
        account_objects::{AddressHistory, CustomerHistory, Pagination},
        exchange_rate_api::ExchangeRateApi,
        wallet_api::WalletManagementApi,
    },
    traits::{
        AccountManagement,
        AuthManagement,
        ExchangeRates,
        NewWalletInfo,
        PaymentGatewayDatabase,
        PaymentGatewayError,
        WalletAuth,
        WalletManagement,
    },
    AccountApi,
    AuthApi,
    OrderFlowApi,
    WalletAuthApi,
};

use crate::{
    auth::{check_login_token_signature, JwtClaims, TokenIssuer},
    config::ProxyConfig,
    data_objects::{
        ExchangeRateResult,
        JsonResponse,
        ModifyOrderParams,
        MoveOrderParams,
        PaymentNotification,
        RoleUpdateRequest,
        TransactionConfirmationNotification,
        UpdateMemoParams,
        UpdatePriceParams,
    },
    errors::ServerError,
    helpers::get_remote_ip,
};

// Web-actix cannot handle generics in handlers, so it's implemented manually using the `route!` macro
#[macro_export]
macro_rules! route {
    ($name:ident => $method:ident $path:literal requires [$($roles:ty),*]) => {
        paste::paste! { pub struct [<$name:camel Route>];}
        paste::paste! {
                impl [<$name:camel Route>] {
                #[allow(clippy::new_without_default)]
                pub fn new() -> Self { Self }
            }
        }
        paste::paste! {
            impl actix_web::dev::HttpServiceFactory for [<$name:camel Route>] {
                fn register(self, config: &mut actix_web::dev::AppService) {
                    let res = actix_web::Resource::new($path)
                        .name(stringify!($name))
                        .guard(actix_web::guard::$method())
                        .to($name)
                        .wrap($crate::middleware::AclMiddlewareFactory::new(&[$($roles),+]));
                    actix_web::dev::HttpServiceFactory::register(res, config);
                }
            }
        }
    };

    ($name:ident => $method:ident $path:literal impl $($bounds:ty),+) => {
        paste::paste! { pub struct [<$name:camel Route>]< $( [< T $bounds:camel> ],)+ >( $( core::marker::PhantomData<fn() -> [< T $bounds:camel> ] >,)+ );}
        paste::paste! { impl< $( [< T $bounds:camel> ],)+ > [<$name:camel Route>]< $( [< T $bounds:camel> ],)+ > {
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self($( core::marker::PhantomData::<fn() -> [< T $bounds:camel> ] >,)+)
            }
        }}
        paste::paste! { impl<$( [< T $bounds:camel >] , )+> actix_web::dev::HttpServiceFactory for [<$name:camel Route>]<$([<T $bounds:camel>],)+>
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
        paste::paste! { pub struct [<$name:camel Route>]<A>(core::marker::PhantomData<fn() -> A>);}
        paste::paste! { impl<A> [<$name:camel Route>]<A> {
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self(core::marker::PhantomData::<fn() -> A>)
            }
        }}
        paste::paste! { impl<A> actix_web::dev::HttpServiceFactory for [<$name:camel Route>]<A>
        where
            A: $($bounds)++ 'static,
        {
            fn register(self, config: &mut actix_web::dev::AppService) {
                let res = actix_web::Resource::new($path)
                    .name(stringify!($name))
                    .guard(actix_web::guard::$method())
                    .to($name::<A>)
                    .wrap($crate::middleware::AclMiddlewareFactory::new(&[$($roles),+]));
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
/// This token is signed by the user('s wallet, typically) and is a JWT with the following fields
/// (See [`tari_payment_engine::db_types::LoginToken`]):
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

//----------------------------------------------   History  ----------------------------------------------------
route!(my_history => Get "/history" impl AccountManagement);
pub async fn my_history<B: AccountManagement>(
    claims: JwtClaims,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET my_history for {}", claims.address);
    let history = get_history_for_address(&claims.address, api.as_ref()).await?;
    Ok(HttpResponse::Ok().json(history))
}

route!(history_for_address => Get "/history/address/{address}" impl AccountManagement where requires [Role::ReadAll]);
pub async fn history_for_address<B: AccountManagement>(
    path: web::Path<SerializedTariAddress>,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let address = path.into_inner().to_address();
    debug!("💻️ GET history for {address}");
    let history = get_history_for_address(&address, api.as_ref()).await?;
    Ok(HttpResponse::Ok().json(history))
}

route!(history_for_customer => Get "/history/customer/{id}" impl AccountManagement where requires [Role::ReadAll]);
pub async fn history_for_customer<B: AccountManagement>(
    path: web::Path<String>,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let id = path.into_inner();
    debug!("💻️ GET history for id {id}");
    let history = get_history_for_customer(&id, api.as_ref()).await?;
    Ok(HttpResponse::Ok().json(history))
}

pub async fn get_history_for_address<B: AccountManagement>(
    address: &TariAddress,
    api: &AccountApi<B>,
) -> Result<AddressHistory, ServerError> {
    let history = api.history_for_address(address).await.map_err(|e| {
        debug!("💻️ Could not fetch account history for {address}. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    Ok(history)
}

pub async fn get_history_for_customer<B: AccountManagement>(
    id: &str,
    api: &AccountApi<B>,
) -> Result<CustomerHistory, ServerError> {
    let history = api.history_for_customer(id).await.map_err(|e| {
        debug!("💻️ Could not fetch account history for account id {id}. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    Ok(history)
}

//----------------------------------------------   Balance  ----------------------------------------------------

route!(my_balance => Get "/balance" impl AccountManagement);
/// Route handler for the balance endpoint
///
/// This route is used to fetch the balance for a given address. The address that is queried is the one that
/// is associated with the JWT token that is supplied in the `tpg_access_token` header.
///
/// To access other balances, the user must have the `ReadAll` role and can use the `/balance/{address}` endpoint.
pub async fn my_balance<B: AccountManagement>(
    claims: JwtClaims,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET my_balance for {}", claims.address);
    get_balance(&claims.address, api.as_ref()).await
}

route!(balance => Get "/balance/{address}" impl AccountManagement where requires [Role::ReadAll]);
/// Route handler for the balance/{address} endpoint
///
/// This route is used to fetch the balance for the address supplied in the query path
pub async fn balance<B: AccountManagement>(
    path: web::Path<SerializedTariAddress>,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let address = path.into_inner().to_address();
    debug!("💻️ GET balance for {address}");
    get_balance(&address, api.as_ref()).await
}

pub async fn get_balance<B: AccountManagement>(
    address: &TariAddress,
    api: &AccountApi<B>,
) -> Result<HttpResponse, ServerError> {
    let balance = api.fetch_address_balance(address).await.map_err(|e| {
        debug!("💻️ Could not fetch balance. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    Ok(HttpResponse::Ok().json(balance))
}

route!(creditors => Get "/creditors" impl AccountManagement where requires [Role::ReadAll]);
/// Route handler for the creditors endpoint
/// Admin users (ReadAll and SuperAdmin roles) can use this endpoint to fetch all accounts that have a positive balance.
/// This is useful for reconciling accounts and ensuring that all payments have been processed.
///
/// The `/api/creditors` endpoint allows admins (with the ReadAll role) to query all accounts that have a positive
/// balance (either pending or current) on the system.
///
/// This is useful for troubleshooting issues when customers have sent a payment but their orders were not matched.
///
/// * Funds might still be in pending and need to be confirmed on the blockchain before the order will be matched. Also
///   check that the hot wallet is sending notifications.
/// * The current balance is not enough the complete the order. In this case there will be both a current balance and a
///   positive value in current orders (did users take fees into account?)
/// * In other cases, the order_id and payment were not matched because of an error in the memos. Here you should see a
///   naked current balance, and some additional sleuthing is required to find the order it corresponds to. Once
///   identified, an admin will need to complete a manual order-payment match.
pub async fn creditors<B: AccountManagement>(api: web::Data<AccountApi<B>>) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET creditors");
    let accounts = api.creditors().await.map_err(|e| {
        debug!("💻️ Could not fetch creditors. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    Ok(HttpResponse::Ok().json(accounts))
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

route!(my_unfulfilled_orders => Get "/unfulfilled_orders" impl AccountManagement);
/// Route handler for my unfulfilled_orders endpoint
///
/// Authenticated users can fetch their own orders using this endpoint. The Tari address for the account is extracted
/// from the JWT token supplied in the `tpg_access_token` header.
///
/// Admin users (ReadAll and SuperAdmin roles) can use the `/unfulfilled_orders/{address}` endpoint to fetch orders for
/// any account.
pub async fn my_unfulfilled_orders<B: AccountManagement>(
    claims: JwtClaims,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET my_unfulfilled_orders for {}", claims.address);
    let address = claims.address;
    let unfulfilled_orders = unfulfilled_orders_for_address(&api, &address).await?;
    Ok(HttpResponse::Ok().json(unfulfilled_orders))
}

async fn unfulfilled_orders_for_address<B: AccountManagement>(
    api: &AccountApi<B>,
    address: &TariAddress,
) -> Result<OrderResult, ServerError> {
    let orders = api.orders_for_address(address).await.map_err(|e| {
        debug!("💻️ Could not fetch my unfulfilled orders. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    let unfulfilled_orders: Vec<Order> = orders
        .orders
        .into_iter()
        .filter(|o| [OrderStatusType::New, OrderStatusType::Unclaimed].contains(&o.status))
        .collect();
    let result = OrderResult {
        address: address.into(),
        total_orders: unfulfilled_orders.iter().map(|o| o.total_price).sum(),
        orders: unfulfilled_orders,
    };
    Ok(result)
}

route!(unfulfilled_orders => Get "/unfulfilled_orders/{address}" impl AccountManagement where requires [Role::ReadAll]);
/// Route handler for the unfulfilled_orders endpoint
///
/// Admins with ReadAll role can use this endpoint to fetch unfulfilled orders for any account.
pub async fn unfulfilled_orders<B: AccountManagement>(
    path: web::Path<SerializedTariAddress>,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let address = path.into_inner().to_address();
    debug!("💻️ GET unfulfilled_orders for {address}");
    let unfulfilled_orders = unfulfilled_orders_for_address(&api, &address).await?;
    Ok(HttpResponse::Ok().json(unfulfilled_orders))
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
    debug!("💻️ GET order_by_id({order_id})");
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
    let result = orders.orders.into_iter().find(|o| o.order_id == order_id);
    Ok(HttpResponse::Ok().json(result))
}

route!(claim_order => Post "/order/claim" impl PaymentGatewayDatabase);
/// Users can claim an order (that is, associate a new order with their wallet address) using the `/order/claim`
/// endpoint.
///
/// This is a `POST` endpoint that requires a JSON body containing a [`MemoSignature`] object.
///
/// This route is unauthenticated
pub async fn claim_order<B: PaymentGatewayDatabase>(
    body: web::Json<MemoSignature>,
    api: web::Data<OrderFlowApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let memo_signature = body.into_inner();
    debug!(
        "💻️ Claim order request for address {} on order {}",
        memo_signature.address.as_address(),
        memo_signature.order_id
    );
    let result = api.claim_order(&memo_signature).await.map_err(|e| {
        debug!("💻️ Order claim failed. {e}");
        e
    })?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_orders<B: AccountManagement>(
    address: &TariAddress,
    api: &AccountApi<B>,
) -> Result<HttpResponse, ServerError> {
    match api.orders_for_address(address).await {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => {
            debug!("💻️ Could not fetch orders. {e}");
            Err(ServerError::BackendError(e.to_string()))
        },
    }
}

route!(customer_ids => Get "/customer_ids" impl AccountManagement where requires [Role::ReadAll]);
/// Utility endpoint to return all customer ids. Pagination is supported.
pub async fn customer_ids<B: AccountManagement>(
    api: web::Data<AccountApi<B>>,
    pagination: web::Query<Pagination>,
) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET customer_ids");
    let customer_ids = api.fetch_customer_ids(pagination.deref()).await.map_err(|e| {
        debug!("💻️ Could not fetch customer ids. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    Ok(HttpResponse::Ok().json(customer_ids))
}

route!(addresses => Get "/addresses" impl AccountManagement where requires [Role::ReadAll]);
/// Utility endpoint to return all addresses. Pagination is supported.
/// Admin users (ReadAll and SuperAdmin roles) can use this endpoint to fetch all addresses on the system.
pub async fn addresses<B: AccountManagement>(
    api: web::Data<AccountApi<B>>,
    pagination: web::Query<Pagination>,
) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET addresses");
    let addresses = api
        .fetch_addresses(pagination.deref())
        .await
        .map_err(|e| {
            debug!("💻️ Could not fetch addresses. {e}");
            ServerError::BackendError(e.to_string())
        })?
        .into_iter()
        .map(SerializedTariAddress::from)
        .collect::<Vec<SerializedTariAddress>>();
    Ok(HttpResponse::Ok().json(addresses))
}

route!(settle_address => Post "/settle/address/{address}" impl PaymentGatewayDatabase where requires [Role::Write]);
pub async fn settle_address<B: PaymentGatewayDatabase>(
    path: web::Path<SerializedTariAddress>,
    api: web::Data<OrderFlowApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let address = path.into_inner().to_address();
    debug!("💻️ GET settle_address for {address}");
    let result = api.settle_orders_for_address(&address).await.map_err(|e| {
        debug!("💻️ Could not settle address. {e}");
        e
    })?;
    Ok(HttpResponse::Ok().json(result))
}

route!(settle_customer => Post "/settle/customer/{customer_id}" impl PaymentGatewayDatabase where requires [Role::Write]);
pub async fn settle_customer<B: PaymentGatewayDatabase>(
    path: web::Path<String>,
    api: web::Data<OrderFlowApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let customer_id = path.into_inner();
    debug!("💻️ GET settle_customer for {customer_id}");
    let result = api.settle_orders_for_customer_id(&customer_id).await.map_err(|e| {
        debug!("💻️ Could not settle customer. {e}");
        e
    })?;
    Ok(HttpResponse::Ok().json(result))
}

route!(settle_my_account => Post "/settle" impl PaymentGatewayDatabase);
pub async fn settle_my_account<B: PaymentGatewayDatabase>(
    claims: JwtClaims,
    api: web::Data<OrderFlowApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let address = claims.address;
    debug!("💻️ GET settle_address for {address}");
    let result = api.settle_orders_for_address(&address).await.map_err(|e| {
        debug!("💻️ Could not settle address. {e}");
        e
    })?;
    Ok(HttpResponse::Ok().json(result))
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

route!(payment_for_order => Get "/payments-for-order/{order_id}" impl AccountManagement where requires [Role::ReadAll]);
/// Route handler for the payments-for-order/{order_id} endpoint
pub async fn payment_for_order<B: AccountManagement>(
    path: web::Path<OrderId>,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let order_id = path.into_inner();
    debug!("💻️ GET payment_for_order({order_id})");
    let payments = api.fetch_payments_for_order(&order_id).await.map_err(|e| {
        debug!("💻️ Could not fetch payments. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    Ok(HttpResponse::Ok().json(payments))
}

//----------------------------------------------   Modify ----------------------------------------------------

route!(issue_credit => Post "/credit" impl PaymentGatewayDatabase where requires [Role::Write]);
/// Route handler for the credit endpoint
/// Admin users (Write role) can use this endpoint to issue a credit note against a customer id.
/// The user's account will be credited, and any eligible orders will immediately be fulfilled.
///
/// Any fulfilled orders will be returned in the response.
pub async fn issue_credit<B: PaymentGatewayDatabase>(
    body: web::Json<CreditNote>,
    api: web::Data<OrderFlowApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let note = body.into_inner();
    debug!("💻️ Credit note request for {note:?}");
    let result = api.issue_credit_note(note).await.map_err(|e| {
        debug!("💻️ Could not issue credit. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    match result {
        Some(orders) => Ok(HttpResponse::Ok().json(orders)),
        None => Ok(HttpResponse::Ok().json(json!({
            "orders_paid": [],
            "settlements": []
        }))),
    }
}

route!(fulfil_order => Post "/fulfill" impl PaymentGatewayDatabase where requires [Role::Write]);
pub async fn fulfil_order<B: PaymentGatewayDatabase>(
    body: web::Json<ModifyOrderParams>,
    api: web::Data<OrderFlowApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let ModifyOrderParams { order_id, reason } = body.into_inner();
    debug!("💻️ Fulfilment request for {order_id} with reason: {reason}");
    let order = api.mark_new_order_as_paid(&order_id, &reason).await.map_err(|e| {
        debug!("💻️ Could not fulfil order. {e}");
        e
    })?;
    Ok(HttpResponse::Ok().json(order))
}

route!(cancel_order => Post "/cancel" impl PaymentGatewayDatabase where requires [Role::Write]);
/// Order cancellation
///
/// Admin users (Write role) can use this endpoint to cancel an order. The order will be marked as cancelled, the
/// user account associated with the order will have its total and current orders value decreased accordingly,
/// and the `OnOrderAnnulled` event will fire (with status [`OrderStatusType::Cancelled`]).
///
/// ## Parameters
/// * `order_id` - The order id to cancel. String.
/// * `reason` - The reason for the cancellation. String.
///
/// ## Returns
/// The cancelled order object.
pub async fn cancel_order<B: PaymentGatewayDatabase>(
    body: web::Json<ModifyOrderParams>,
    api: web::Data<OrderFlowApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let ModifyOrderParams { order_id, reason } = body.into_inner();
    info!("💻️ Cancel order request for {order_id}. Reason: {reason}");
    let order = api.cancel_or_expire_order(&order_id, OrderStatusType::Cancelled, &reason).await.map_err(|e| {
        debug!("💻️ Could not cancel order. {e}");
        e
    })?;
    Ok(HttpResponse::Ok().json(order))
}

route!(update_order_memo => Patch "/order_memo" impl PaymentGatewayDatabase where requires [Role::Write]);
/// Update an order's memo field.
///
/// Admin users (Write role) can use this endpoint to update an order's memo field.
/// *Note*: the HTTP method used for this endpoint is PATCH, rather than POST.
///
/// The side effects of this call are:
/// * The memo is updated
/// * An `OrderModifiedEvent` is triggered.
/// * An audit log entry is added.
///
/// The memo is *not* checked for a valid signature and an order matching
/// cycle is not fired.
///
/// If a user has messed up the memo field, then we recommend cancelling the
/// order and asking the user to try again.
///
/// If this becomes cumbersome, and there's a clean flow for admins helping
/// provide a valid order signature, then we can modify this endpoint to do
/// so. Right now, it's not clear whether the UX would be any better than
/// re-doing the order.
pub async fn update_order_memo<B: PaymentGatewayDatabase>(
    body: web::Json<UpdateMemoParams>,
    api: web::Data<OrderFlowApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let UpdateMemoParams { order_id, new_memo, reason } = body.into_inner();
    let reason = reason.unwrap_or_else(|| "No reason provided".to_string());
    info!("💻️ Update order memos request for {order_id}. Reason: {reason}");
    let order = api.update_memo_for_order(&order_id, &new_memo).await.map_err(|e| {
        debug!("💻️ Could not update order memo. {e}");
        e
    })?;
    Ok(HttpResponse::Ok().json(order))
}

route!(update_price => Patch "/order_price" impl PaymentGatewayDatabase where requires [Role::Write]);
/// Provides an endpoint for admins to adjust the price of an order.
///
/// Admins can call PATCH /api/order_price with the order_id, new price, and
/// a reason to adjust the price of an order up or down.
///
/// If the price decreases such that an existing balance in the user's
/// account will be able to fill the order, then the order will
/// automatically be filled.
///
/// The new price must be positive.
pub async fn update_price<B: PaymentGatewayDatabase>(
    body: web::Json<UpdatePriceParams>,
    api: web::Data<OrderFlowApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let UpdatePriceParams { order_id, new_price, reason } = body.into_inner();
    let reason = reason.unwrap_or_else(|| "No reason provided".to_string());
    info!("💻️ Update order price request for {order_id}. Reason: {reason}");
    let order = api.update_price_for_order(&order_id, new_price).await.map_err(|e| {
        debug!("💻️ Could not update order price. {e}");
        e
    })?;
    Ok(HttpResponse::Ok().json(order))
}

route!(reassign_order => Patch "/reassign_order" impl PaymentGatewayDatabase where requires [Role::Write]);
/// Provides an endpoint for admins to adjust the price of an order.
/// Admins can call `PATCH /api/reassign_order` with the order_id, new customer_id, and a reason to reassign an order
/// to a different customer.
///
/// If that customer has a credit balance in excess of the order price, the order will be automatically fulfilled.
///
/// The endpoint returns a `OrderMovedResult` JSON object:
/// ```json
/// {
///     "orders": { "new_order": {}, "old_order": {} },
///     "old_account_id": 1000,
///     "new_account_id": 1200,
///     "is_filled": false
///  }
/// ```
pub async fn reassign_order<B: PaymentGatewayDatabase>(
    body: web::Json<MoveOrderParams>,
    api: web::Data<OrderFlowApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let MoveOrderParams { order_id, new_customer_id, reason } = body.into_inner();
    info!("💻️ Assigning existing order {order_id} to customer {new_customer_id}. Reason: {reason}");
    let order = api.assign_order_to_new_customer(&order_id, &new_customer_id).await.map_err(|e| {
        debug!("💻️ Could not assign order. {e}");
        e
    })?;
    Ok(HttpResponse::Ok().json(order))
}

route!(reset_order => Patch "/reset_order/{order_id}" impl PaymentGatewayDatabase where requires [Role::Write]);
/// Provides an endpoint for admins to reset an order to the `New` state.
///
/// `reset_order` is a PATCH HTTP method.
///
/// This is useful when an order has expired or was cancelled, but the customer still wants to pay for it, or
/// if an order was reassigned, or otherwise modified and needs to be re-processed.
///
/// ## Arguments
/// Arguments for this route are set on the path, i.e.
/// `/reset_order/{order_id}`
///
/// ## Returns
/// The endpoint returns the order states before and after the reset.
#[cfg(not(feature = "shopify"))]
pub async fn reset_order<B: PaymentGatewayDatabase>(
    path: web::Path<OrderId>,
    api: web::Data<OrderFlowApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let order_id = path.into_inner();
    info!("💻️ Resetting order {order_id}");
    let updated_order = api.reset_order(&order_id).await.map_err(|e| {
        debug!("💻️ Could not reset order. {e}");
        e
    })?;
    Ok(HttpResponse::Ok().json(updated_order))
}

#[cfg(feature = "shopify")]
pub async fn reset_order<B: PaymentGatewayDatabase>() -> Result<HttpResponse, ServerError> {
    Err(ServerError::UnsupportedAction("Resetting orders is not supported in Shopify".to_string()))
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
    let PaymentNotification { mut payment, auth } = body.into_inner();
    let use_x_forwarded_for = config.use_x_forwarded_for;
    let use_forwarded = config.use_forwarded;
    let disable_whitelist = config.disable_wallet_whitelist;
    // Log the payment
    trace!("💻️ Extracting remote IP address. {req:?}. {:?}", req.connection_info());
    let peer_addr = match (get_remote_ip(&req, use_x_forwarded_for, use_forwarded), disable_whitelist) {
        (Some(ip), _) => Some(ip),
        (None, true) => {
            info!(
                "💻️ Could not determine remote IP address for a wallet payment notification. The whitelist is \
                 disabled, so the request is allowed, but it could not be logged."
            );
            None
        },
        (None, false) => {
            warn!(
                "💻️ Could not determine remote IP address for a wallet payment notification. The request is rejected"
            );
            return HttpResponse::Unauthorized().finish();
        },
    };
    info!("💻️ New payment notification received from IP {peer_addr:?}.");
    info!("💻️ Payment: {}", serde_json::to_string(&payment).unwrap_or_else(|e| format!("{e}")));
    info!("💻️ Auth: {}", serde_json::to_string(&auth).unwrap_or_else(|e| format!("{e}")));
    trace!("💻️ Verifying wallet signature");
    if !auth.is_valid(&payment) {
        warn!("💻️ Invalid wallet signature received from {peer_addr:?}. The request is rejected.");
        return HttpResponse::Unauthorized().finish();
    }
    let auth_api = auth_api.as_ref();
    if let Err(e) = auth_api.authenticate_wallet(auth, peer_addr.as_ref(), &payment, disable_whitelist).await {
        warn!("💻️ Unauthorized wallet signature received from {peer_addr:?}. Reason: {e}. The request is rejected.");
        return HttpResponse::Unauthorized().finish();
    }
    // -- from here on, we trust that the notification is legitimate.
    // -- extract the order_id from the memo signature, if present
    match payment.try_extract_order_id() {
        Some(true) => {
            let id = payment.order_id.as_ref().map(|o| o.as_str()).unwrap_or_else(|| "??");
            info!("💻️ Payment memo contains a valid claim for order {id}");
        },
        Some(false) => debug!("💻️ Payment memo does not contain a valid claim for an order."),
        None => debug!("💻️ Payment memo was empty and did thus did not contain a claim for an order"),
    }
    let result = match order_api.process_new_payment(payment).await {
        Ok(payment) => {
            info!("💻️ Incoming payment processed successfully for {}.", payment.sender);
            let body = serde_json::to_string(&payment).unwrap_or_else(|e| format!("{e}"));
            JsonResponse::success(body)
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

route!(tx_confirmation_notification => Post "/tx_confirmation" impl PaymentGatewayDatabase, WalletAuth );
pub async fn tx_confirmation_notification<BOrder, BAuth>(
    req: HttpRequest,
    config: web::Data<ProxyConfig>,
    auth_api: web::Data<WalletAuthApi<BAuth>>,
    order_api: web::Data<OrderFlowApi<BOrder>>,
    body: web::Json<TransactionConfirmationNotification>,
) -> HttpResponse
where
    BAuth: WalletAuth,
    BOrder: PaymentGatewayDatabase,
{
    trace!("💻️ Received transaction confirmation notification");
    let TransactionConfirmationNotification { confirmation, auth } = body.into_inner();
    let use_x_forwarded_for = config.use_x_forwarded_for;
    let use_forwarded = config.use_forwarded;
    let disable_whitelist = config.disable_wallet_whitelist;
    trace!("💻️ Extracting remote IP address. {req:?}. {:?}", req.connection_info());
    let peer_addr = match (get_remote_ip(&req, use_x_forwarded_for, use_forwarded), disable_whitelist) {
        (Some(ip), _) => Some(ip),
        (None, true) => {
            info!(
                "💻️ Could not determine remote IP address for a wallet payment confirmation. The whitelist is \
                 disabled, so the request is allowed, but it could not be logged."
            );
            None
        },
        (None, false) => {
            warn!(
                "💻️ Could not determine remote IP address for a wallet payment confirmation. The request is rejected"
            );
            return HttpResponse::Unauthorized().finish();
        },
    };
    // Log the payment
    info!("💻️ New transaction confirmation received from IP {peer_addr:?}.");
    info!("💻️ Confirmation: {}", serde_json::to_string(&confirmation).unwrap_or_else(|e| format!("{e}")));
    info!("💻️ Auth: {}", serde_json::to_string(&auth).unwrap_or_else(|e| format!("{e}")));
    trace!("💻️ Verifying wallet signature");
    if !auth.is_valid(&confirmation) {
        warn!("💻️ Invalid wallet signature received from {peer_addr:?}. The request is rejected.");
        return HttpResponse::Unauthorized().finish();
    }
    let auth_api = auth_api.as_ref();
    if let Err(e) = auth_api.authenticate_wallet(auth, peer_addr.as_ref(), &confirmation, disable_whitelist).await {
        warn!("💻️ Unauthorized wallet signature received from {peer_addr:?}. Reason: {e}. The request is rejected.");
        return HttpResponse::Unauthorized().finish();
    }
    // -- from here on, we trust that the notification is legitimate.
    let tx_id = confirmation.txid.clone();
    let result = match order_api.confirm_payment(confirmation.txid).await {
        Err(PaymentGatewayError::PaymentModificationNoOp) => {
            info!("💻️ Payment {} already confirmed.", tx_id);
            JsonResponse::success("Payment already confirmed.")
        },
        Err(e) => {
            error!("💻️ Could not confirm payment. {e}");
            JsonResponse::failure(String::from("Could not confirm payment."))
        },
        Ok(payment) => {
            info!("💻️ Payment {} confirmed successfully.", payment.txid);
            debug!("💻️ Payment details: {payment:?}");
            JsonResponse::success(format!("Payment {tx_id} confirmed successfully."))
        },
    };
    HttpResponse::Ok().json(result)
}

//----------------------------------------------   SuperAdmin  ----------------------------------------------------
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

route!(get_authorized_wallets => Get "/wallets" impl WalletManagement where requires [Role::ReadAll]);
/// Get all wallets that are authorized to receive funds on behalf of the payment gateway.
///
/// This endpoint is only accessible to users with the `ReadAll` role.
pub async fn get_authorized_wallets<W: WalletManagement>(
    api: web::Data<WalletManagementApi<W>>,
) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET wallets");
    let wallets = api.fetch_authorized_wallets().await.map_err(|e| {
        debug!("💻️ Could not fetch wallets. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    Ok(HttpResponse::Ok().json(wallets))
}

route!(get_authorized_addresses => Get "/send_to" impl WalletManagement);
/// Get all wallet addresses that are authorized to receive funds on behalf of the payment gateway.
///
/// Only addresses are returned. IP addresses and nonces are not included.
///
/// This is a publicly accessible endpoint.
pub async fn get_authorized_addresses<W: WalletManagement>(
    api: web::Data<WalletManagementApi<W>>,
) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET wallets");
    let wallets = api
        .fetch_authorized_wallets()
        .await
        .map_err(|e| {
            debug!("💻️ Could not fetch wallets. {e}");
            ServerError::BackendError(e.to_string())
        })?
        .into_iter()
        .map(|w| w.address)
        .collect::<Vec<_>>();
    Ok(HttpResponse::Ok().json(wallets))
}

route!(remove_authorized_wallet => Delete "/wallets/{address}" impl WalletManagement where requires [Role::SuperAdmin]);
/// Remove a wallet from the list of authorized wallets.
/// This endpoint is only accessible to users with the `SuperAdmin` role.
pub async fn remove_authorized_wallet<W: WalletManagement>(
    api: web::Data<WalletManagementApi<W>>,
    address: web::Path<SerializedTariAddress>,
) -> Result<HttpResponse, ServerError> {
    let address = address.into_inner().to_address();
    debug!("💻️ DELETE wallet {address}");
    api.deregister_wallet(&address).await.map_err(|e| {
        info!("💻️ Could not remove wallet. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    Ok(HttpResponse::Ok().finish())
}

route!(add_authorized_wallet => Post "/wallets" impl WalletManagement where requires [Role::SuperAdmin]);
/// Add a wallet to the list of authorized wallets.
/// This endpoint is only accessible to users with the `SuperAdmin` role.
pub async fn add_authorized_wallet<W: WalletManagement>(
    api: web::Data<WalletManagementApi<W>>,
    body: web::Json<NewWalletInfo>,
) -> Result<HttpResponse, ServerError> {
    let wallet = body.into_inner();
    debug!("💻️ POST authorize_new_wallet {}", wallet.address.as_base58());
    api.register_wallet(wallet).await.map_err(|e| {
        info!("💻️ Could not add wallet. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    Ok(HttpResponse::Ok().finish())
}

//----------------------------------------------  Check Token  ----------------------------------------------------
route!(check_token => Get "/check_token" requires [Role::User]);
pub async fn check_token(claims: JwtClaims) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET check_token for {}", claims.address);
    Ok(HttpResponse::Ok().body("Token is valid."))
}

//----------------------------------------------   Exchange rates  ----------------------------------------------------
route!(get_exchange_rate => Get "/exchange_rate/{currency}" impl ExchangeRates where requires [Role::ReadAll]);
pub async fn get_exchange_rate<B: ExchangeRates>(
    currency: web::Path<String>,
    api: web::Data<ExchangeRateApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let cur = currency.into_inner();
    let rate = api.fetch_last_rate(cur.as_str()).await.map_err(|e| {
        debug!("💻️ Could not fetch exchange rate. {e}");
        ServerError::BackendError(e.to_string())
    })?;
    let rate = ExchangeRateResult::from(rate);
    Ok(HttpResponse::Ok().json(rate))
}
