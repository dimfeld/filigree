use axum::{extract::Request, response::Response};
use futures_util::future::BoxFuture;
use sqlx::PgPool;
use tower::{Layer, Service};

#[derive(Clone)]
struct AuthLayer {
    // TODO replace with some struct that implements a trait for getting the user data
    pool: PgPool,
}

impl AuthLayer {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            pool: self.pool.clone(),
            inner,
        }
    }
}

#[derive(Clone)]
pub struct AuthService<S> {
    pool: PgPool,
    inner: S,
}

impl<S> Service<Request> for AuthService<S>
where
    S: Service<Request, Response = Response> + Send + Clone + 'static,
    S::Response: 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let clone = self.inner.clone();
        // See https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            // TODO install tower-session and use for session cookie
            // TODO do all the lookups

            let response = inner.call(request).await?;
            Ok(response)
        })
    }
}
