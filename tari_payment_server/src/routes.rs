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

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use log::*;
use paste::paste;
use tari_common_types::tari_address::TariAddress;
use tari_payment_engine::{db_types::Role, AccountApi, AccountManagement, AuthApi, AuthManagement};

use crate::{
    auth::{check_login_token_signature, JwtClaims, TokenIssuer},
    data_objects::{RoleUpdateRequest, SerializedTariAddress},
    errors::ServerError,
    shopify_order::ShopifyOrder,
};

#[get("/health")]
pub async fn health() -> impl Responder {
    trace!("💻️ Received health check request");
    HttpResponse::Ok().body("👍️\n")
}

// Web-actix cannot handle generics in handlers, so it's implemented manually using the `route!` macro
macro_rules! route {
    ($name:ident => $method:ident $path:literal impl $($bounds:ty),+) => {
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
                    .to($name::<A>);
                actix_web::dev::HttpServiceFactory::register(res, config);
            }
        }}
    };

    ($name:ident => $method:ident $path:literal impl $($bounds:ty),+ where requires [$($roles:ty),+])  => {
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
/// This route is used to fetch account information for a given address. The address that is queried is the one that
/// is supplied in the path.
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

//#[get("/orders")]
route!(my_orders => Get "/orders" impl AccountManagement);
pub async fn my_orders<B: AccountManagement>(
    claims: JwtClaims,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    debug!("💻️ GET my_orders for {}", claims.address);
    get_orders(&claims.address, api.as_ref()).await
}

//#[get("/orders/{address}")]
route!(orders => Get "/orders/{address}" impl AccountManagement where requires [Role::ReadAll]);
pub async fn orders<B: AccountManagement>(
    path: web::Path<SerializedTariAddress>,
    api: web::Data<AccountApi<B>>,
) -> Result<HttpResponse, ServerError> {
    let address = path.into_inner().to_address();
    debug!("💻️ GET orders for {address}");
    get_orders(&address, api.as_ref()).await
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

#[post("/webhook/checkout_create")]
pub async fn shopify_webhook(req: HttpRequest, body: web::Bytes) -> Result<HttpResponse, ServerError> {
    trace!("💻️ Received webhook request: {}", req.uri());
    let payload = std::str::from_utf8(body.as_ref()).map_err(|e| ServerError::InvalidRequestBody(e.to_string()))?;
    trace!("💻️ Decoded payload body. {} bytes", payload.bytes().len());
    let _order: ShopifyOrder = serde_json::from_str(payload).map_err(|e| {
        error!("💻️ Could not deserialize order payload. {e}");
        debug!("💻️ JSON payload: {payload}");
        ServerError::CouldNotDeserializePayload
    })?;
    // let new_order = ShopifyOrder::try_from(order)?;
    // TODO - Send the new order to payment engine

    Ok(HttpResponse::Ok().finish())
}

//#[post("/roles/")]
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
