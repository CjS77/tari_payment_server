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
use crate::spg_server::errors::ServerError;
use crate::spg_server::new_order::FreshOrder;
use crate::spg_server::new_order_service::{NewOrder, NewOrderService};
use actix::prelude::*;
use actix_web::{get, post, web, web::Data, HttpRequest, HttpResponse, Responder};
use log::*;

#[get("/health")]
pub async fn health() -> impl Responder {
    trace!("💻 Received health check request");
    HttpResponse::Ok().body("👍\n")
}

#[post("/webhook/checkout_create")]
pub async fn shopify_webhook(
    req: HttpRequest,
    body: web::Bytes,
    new_order_actor: Data<Addr<NewOrderService>>,
) -> Result<HttpResponse, ServerError> {
    trace!("💻 Received webhook request: {}", req.uri());
    let payload = std::str::from_utf8(body.as_ref())
        .map_err(|e| ServerError::InvalidRequestBody(e.to_string()))?;
    trace!("💻 Decoded payload body. {} bytes", payload.bytes().len());
    let order: FreshOrder = serde_json::from_str(payload).map_err(|e| {
        error!("💻 Could not deserialize order payload. {e}");
        debug!("💻 JSON payload: {payload}");
        ServerError::CouldNotDeserializePayload
    })?;
    let addr = new_order_actor.get_ref().clone();
    dispatch_event_to_subscribers(addr, order)?;
    Ok(HttpResponse::Ok().finish())
}

pub fn dispatch_event_to_subscribers(
    actor: Addr<NewOrderService>,
    payload: FreshOrder,
) -> Result<(), ServerError> {
    debug!("💻 Forwarding NewOrder {} to NewOrderService", payload.id);
    let order = NewOrder(payload);
    match actor.try_send(order) {
        Err(SendError::Full(_)) => {
            warn!("💻 Subscriber message queue is full");
            Err(ServerError::MailboxFull)
        }
        Err(SendError::Closed(_)) => {
            warn!("💻 Subscriber message queue is closed");
            Err(ServerError::MailboxClosed)
        }
        Ok(()) => {
            debug!("💻 New order message was sent ok.");
            Ok(())
        }
    }
}
