//! HMAC middleware for Actix Web.
//!
//! This module provides a middleware for Actix Web that checks the HMAC signature of incoming requests.
//!
//! Shopify sends a HMAC signature in the headers of the request, using the `TPG_SHOPIFY_ADMIN_ACCESS_TOKEN` as the
//! secret key, and the body of the request as the data to sign.
//!
//! The HMAC is provided in the `X-Shopify-Hmac-SHA256` header.
//!
//! You can use this middleware to verify the HMAC signature of incoming requests by wrapping all shopify webhook
//! calls with this middleware.

use std::{
    future::{ready, Ready},
    rc::Rc,
};

use actix_http::h1;
use actix_web::{
    dev::{forward_ready, Payload, Service, ServiceRequest, ServiceResponse, Transform},
    error::{ErrorBadRequest, ErrorForbidden},
    web,
    Error,
};
use futures::future::LocalBoxFuture;
use log::{trace, warn};
use tpg_common::Secret;

use crate::helpers::calculate_hmac;

pub struct HmacMiddlewareFactory {
    hmac_header: String,
    key: Secret<String>,
    // If false, then the middleware will not check the HMAC signature and always allow the call
    enabled: bool,
}

impl HmacMiddlewareFactory {
    pub fn new(hmac_header: &str, key: Secret<String>, enabled: bool) -> Self {
        HmacMiddlewareFactory { hmac_header: hmac_header.into(), key, enabled }
    }
}

impl<S, B> Transform<S, ServiceRequest> for HmacMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Error = Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type InitError = ();
    type Response = ServiceResponse<B>;
    type Transform = HmacMiddlewareService<S>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(HmacMiddlewareService {
            hmac_header: self.hmac_header.clone(),
            key: self.key.clone(),
            enabled: self.enabled,
            service: Rc::new(service),
        }))
    }
}

pub struct HmacMiddlewareService<S> {
    hmac_header: String,
    key: Secret<String>,
    enabled: bool,
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for HmacMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = ServiceResponse<B>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let secret = self.key.reveal().clone();
        let hmac_header = self.hmac_header.clone();
        let enabled = self.enabled;
        Box::pin(async move {
            trace!("🔐️ Checking HMAC for request");
            if !enabled {
                trace!("🔐️ HMAC checks are disabled. Allowing request.");
                return service.call(req).await;
            }
            let data = req.extract::<web::Bytes>().await.map_err(|e| {
                warn!("🔐️ Failed to extract request data: {:?}", e);
                ErrorBadRequest("Failed to extract request data.")
            })?;
            let hmac_calc = calculate_hmac(&secret, data.as_ref());
            let hmac = req.headers().get(&hmac_header).ok_or_else(|| {
                warn!("No HMAC signature found in request. denying access.");
                ErrorForbidden("No HMAC signature found.")
            })?;
            let validated = hmac == hmac_calc.as_str();
            if validated {
                trace!("🔐️ HMAC check for request ✅️");
                req.set_payload(bytes_to_payload(data));
                service.call(req).await
            } else {
                warn!("🔐️ Invalid HMAC signature found in request. denying access.");
                Err(ErrorForbidden("Invalid HMAC signature."))
            }
        })
    }
}

fn bytes_to_payload(buf: web::Bytes) -> Payload {
    let (_, mut pl) = h1::Payload::create(true);
    pl.unread_data(buf);
    Payload::from(pl)
}
