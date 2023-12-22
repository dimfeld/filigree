use std::sync::Arc;

use axum::{extract::Request, response::Response};
use tower::{Layer, Service};

use super::{lookup::AuthLookup, AuthInfo, AuthQueries};

/// A type-erased container for AuthQueries
pub type AuthQueriesContainer<INFO> = Arc<dyn AuthQueries<AuthInfo = INFO>>;

/// A layer that inserts the auth lookup object into the request, for later
/// use by the Authed extractor.
#[derive(Clone)]
pub struct AuthLayer<INFO: AuthInfo> {
    queries: AuthQueriesContainer<INFO>,
}

impl<INFO: AuthInfo> AuthLayer<INFO> {
    /// Create a new AuthLayer with the provided lookup object
    pub fn new(queries: AuthQueriesContainer<INFO>) -> Self {
        Self { queries }
    }
}

impl<S, INFO: AuthInfo> Layer<S> for AuthLayer<INFO> {
    type Service = AuthService<S, INFO>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            queries: self.queries.clone(),
            inner,
        }
    }
}

/// A middleware service for fetching authorization info
#[derive(Clone)]
pub struct AuthService<S, INFO: AuthInfo> {
    queries: AuthQueriesContainer<INFO>,
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
        let lookup = AuthLookup::new(self.queries.clone());
        request.extensions_mut().insert(Arc::new(lookup));
        self.inner.call(request)
    }
}
