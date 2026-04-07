use actix_web::{
    Error as ActixError, HttpMessage, HttpRequest,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use std::{
    future::{Future, Ready, ready},
    pin::Pin,
    rc::Rc,
};

use log::{info, warn};

use crate::ports::Error;

/// Middleware that validates API key for protected endpoints
pub struct ApiKeyAuth {
    api_key: Rc<String>,
}

impl ApiKeyAuth {
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self {
            api_key: Rc::new(api_key),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for ApiKeyAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type InitError = ();
    type Transform = ApiKeyAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiKeyAuthMiddleware {
            service: Rc::new(service),
            api_key: self.api_key.clone(),
        }))
    }
}

pub struct ApiKeyAuthMiddleware<S> {
    service: Rc<S>,
    api_key: Rc<String>,
}

impl<S, B> Service<ServiceRequest> for ApiKeyAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let expected_key = self.api_key.clone();
        let service = self.service.clone();

        Box::pin(async move {
            // Extract API key from Authorization header
            let provided_key = req
                .headers()
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .unwrap_or("");

            // Validate API key
            if provided_key.is_empty() || provided_key != expected_key.as_str() {
                warn!(
                    "Auth REJECTED for {} {} (key: '{}')",
                    req.method(),
                    req.path(),
                    if provided_key.is_empty() {
                        "<empty>"
                    } else {
                        "<invalid>"
                    }
                );
                let error = Error::Unauthorized("Invalid or missing API key".to_string());
                return Err(actix_web::error::ErrorUnauthorized(error));
            }

            info!("Auth OK for {} {}", req.method(), req.path());
            // Store the authenticated flag in request extensions for later use
            req.extensions_mut().insert(Authenticated);

            service.call(req).await
        })
    }
}

/// Marker type to indicate request has been authenticated
#[derive(Clone)]
pub struct Authenticated;

/// Helper to check if a request is authenticated
#[must_use]
pub fn is_authenticated(req: &HttpRequest) -> bool {
    req.extensions().get::<Authenticated>().is_some()
}
