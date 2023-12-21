use std::sync::Arc;

use axum::{extract::Request, response::Response};
use tower::{Layer, Service};

use super::{
    lookup::{AuthLookup, AuthLookupOptions},
    AuthInfo,
};

/// A layer that inserts the auth lookup object into the request, for later
/// use by the Authed extractor.
#[derive(Clone)]
pub struct AuthLayer<INFO: AuthInfo> {
    lookup: Arc<AuthLookup<INFO>>,
}

impl<INFO: AuthInfo> AuthLayer<INFO> {
    /// Create a new AuthLayer with the provided lookup object
    pub fn new(lookup: Arc<AuthLookup<INFO>>) -> Self {
        Self { lookup }
    }
}

impl<S, INFO: AuthInfo> Layer<S> for AuthLayer<INFO> {
    type Service = AuthService<S, INFO>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            lookup: self.lookup.clone(),
            inner,
        }
    }
}

/// A middleware service for fetching authorization info
#[derive(Clone)]
pub struct AuthService<S, INFO: AuthInfo> {
    lookup: Arc<AuthLookup<INFO>>,
    inner: S,
}

impl<S, INFO: AuthInfo + 'static> Service<Request> for AuthService<S, INFO>
where
    S: Service<Request, Response = Response> + Send + Clone + 'static,
    S::Response: 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request) -> Self::Future {
        request.extensions_mut().insert(self.lookup.clone());
        self.inner.call(request)
    }
}

/// Create a new [AuthLayer] containing an [AuthLookup] built from the given options.
pub fn auth_layer<INFO: AuthInfo>(lookup_options: AuthLookupOptions) -> AuthLayer<INFO> {
    let lookup = Arc::new(AuthLookup::new(lookup_options));
    AuthLayer::new(lookup)
}
