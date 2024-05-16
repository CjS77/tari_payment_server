//! Access control list middleware for the Tari Payment Server.
//! This middleware can be placed on any route or service.
//!
//! It will check the incoming request for a valid JWT token and then check the claims in the token against the required
//! roles for the route. If the token is valid and the user has the required roles, the request will be allowed to
//! continue. Otherwise, a 403 Forbidden response will be returned.

use std::rc::Rc;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::{ErrorForbidden, ErrorInternalServerError},
    Error,
    HttpMessage,
};
use futures::{
    future::{ok, LocalBoxFuture, Ready},
    FutureExt,
};
use log::*;
use tari_payment_engine::db_types::Role;

use crate::auth::JwtClaims;

pub struct AclMiddlewareFactory {
    required_roles: Vec<Role>,
}

impl AclMiddlewareFactory {
    pub fn new(required_roles: &[Role]) -> Self {
        AclMiddlewareFactory { required_roles: required_roles.to_vec() }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AclMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Error = Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type InitError = ();
    type Response = ServiceResponse<B>;
    type Transform = AclMiddlewareService<S>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AclMiddlewareService { required_roles: self.required_roles.clone(), service: Rc::new(service) })
    }
}

pub struct AclMiddlewareService<S> {
    required_roles: Vec<Role>,
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AclMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = ServiceResponse<B>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let required_roles = self.required_roles.clone();
        async move {
            trace!("🔐️ Checking ACL for request");
            let jwt_claims = req
                .extensions()
                .get::<JwtClaims>()
                .ok_or_else(|| {
                    let ip = req.peer_addr().map(|a| a.to_string()).unwrap_or_else(|| "unknown".to_string());
                    warn!("🔐️ No JWT claims found for request: {}. IP: {ip}", req.uri());
                    ErrorInternalServerError("No JWT claims found in request extensions")
                })?
                .clone();
            // SuperAdmin can access any route
            let mut approved = jwt_claims.roles.contains(&Role::SuperAdmin);
            if approved {
                info!("🔐️ SuperAdmin access granted to {} for {}", jwt_claims.address, req.uri());
            }
            approved |= required_roles.iter().all(|role| jwt_claims.roles.contains(role));
            if approved {
                service.call(req).await
            } else {
                warn!("🔐️ User '{}' did not have necessary permissions for {}", jwt_claims.address, req.uri());
                Err(ErrorForbidden("Insufficient permissions."))
            }
        }
        .boxed_local()
    }
}
