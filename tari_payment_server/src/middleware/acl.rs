//! Access control list middleware for the Tari Payment Server.
//! This middleware can be placed on any route or service.
//!
//! It will check the incoming request for a valid JWT token and then check the claims in the token against the required
//! roles for the route. If the token is valid and the user has the required roles, the request will be allowed to
//! continue. Otherwise, a 403 Forbidden response will be returned.

use std::pin::Pin;
use std::rc::Rc;
use actix_web::dev::{forward_ready, Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpMessage};
use actix_web::error::{ErrorForbidden, ErrorInternalServerError};
use futures::future::{ok, Ready};
use futures::Future;
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
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AclMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AclMiddlewareService {
            required_roles: self.required_roles.clone(),
            service: Rc::new(service),
        })
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
        B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let required_roles = self.required_roles.clone();
        Box::pin( async move {
            let jwt_claims = req.extensions().get::<JwtClaims>()
                .ok_or_else(|| {
                    log::warn!("No JWT claims found in request extensions");
                    ErrorInternalServerError("No JWT claims found in request extensions")
                })?.clone();
            if required_roles.iter().all(|role| jwt_claims.roles.contains(role)) {
                service.call(req).await
            } else {
                Err(ErrorForbidden("Insufficient permissions"))
            }
        })
    }
}


