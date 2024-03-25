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
use crate::auth::{check_login_token_signature, TokenIssuer};
use crate::errors::ServerError;
use crate::shopify_order::ShopifyOrder;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use log::*;
use std::marker::PhantomData;
use tari_payment_engine::{AuthApi, AuthManagement};

#[get("/health")]
pub async fn health() -> impl Responder {
    trace!("💻️ Received health check request");
    HttpResponse::Ok().body("👍️\n")
}

// Web-actix cannot handle generics in handlers, so it's implemented manually using `AuthRoute`
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
    let payload = req
        .headers()
        .get("Authorization")
        .ok_or(ServerError::CouldNotDeserializeAuthToken)?;
    let login_token = payload.to_str().map_err(|e| {
        debug!("💻️ Could not read auth token. {e}");
        ServerError::CouldNotDeserializeAuthToken
    })?;
    let token = check_login_token_signature(login_token)?;
    debug!("💻️ Login token was validated for {token:?}");
    let cust_id = api
        .update_nonce_for_address(&token.address, token.nonce)
        .await?;
    let access_token = signer.issue_token(cust_id, token, None)?;
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(access_token))
}

pub struct AuthRoute<A>(PhantomData<fn() -> A>);
impl<A> AuthRoute<A> {
    pub fn new() -> Self {
        Self(PhantomData::<fn() -> A>)
    }
}
impl<A> actix_web::dev::HttpServiceFactory for AuthRoute<A>
where
    A: AuthManagement + 'static,
{
    fn register(self, config: &mut actix_web::dev::AppService) {
        let res = actix_web::Resource::new("/auth")
            .name("auth")
            .guard(actix_web::guard::Post())
            .to(auth::<A>);
        actix_web::dev::HttpServiceFactory::register(res, config);
    }
}

#[post("/webhook/checkout_create")]
pub async fn shopify_webhook(
    req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, ServerError> {
    trace!("💻️ Received webhook request: {}", req.uri());
    let payload = std::str::from_utf8(body.as_ref())
        .map_err(|e| ServerError::InvalidRequestBody(e.to_string()))?;
    trace!("💻️ Decoded payload body. {} bytes", payload.bytes().len());
    let _order: ShopifyOrder = serde_json::from_str(payload).map_err(|e| {
        error!("💻️ Could not deserialize order payload. {e}");
        debug!("💻️ JSON payload: {payload}");
        ServerError::CouldNotDeserializePayload
    })?;
    //let new_order = ShopifyOrder::try_from(order)?;
    // TODO - Send the new order to payment engine

    Ok(HttpResponse::Ok().finish())
}
