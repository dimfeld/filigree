use std::{borrow::Cow, marker::PhantomData};

use axum::{extract::Request, response::IntoResponse};
use futures_util::future::BoxFuture;
use tower::{Layer, Service};

use super::{get_auth_info, AuthError, AuthInfo};

/// Generate a middleware layer that checks if the user has a particular permission
pub fn has_permission<INFO: AuthInfo>(s: impl Into<Cow<'static, str>>) -> HasPermissionLayer<INFO> {
    HasPermissionLayer {
        permission: s.into(),
        _marker: PhantomData::default(),
    }
}

/// Middleware layer that checks if the user has a particular permission
pub struct HasPermissionLayer<INFO: AuthInfo> {
    permission: Cow<'static, str>,
    _marker: PhantomData<INFO>,
}

impl<INFO: AuthInfo> Clone for HasPermissionLayer<INFO> {
    fn clone(&self) -> Self {
        Self {
            permission: self.permission.clone(),
            _marker: PhantomData::default(),
        }
    }
}

impl<S, INFO: AuthInfo> Layer<S> for HasPermissionLayer<INFO> {
    type Service = HasPermissionService<S, INFO>;

    fn layer(&self, inner: S) -> Self::Service {
        HasPermissionService {
            permission: self.permission.clone(),
            inner,
            _marker: PhantomData::default(),
        }
    }
}

/// The middleware service for checking if the user has a particular permission
pub struct HasPermissionService<S, INFO: AuthInfo> {
    permission: Cow<'static, str>,
    inner: S,
    _marker: PhantomData<INFO>,
}

impl<S: Clone, INFO: AuthInfo> Clone for HasPermissionService<S, INFO> {
    fn clone(&self) -> Self {
        Self {
            permission: self.permission.clone(),
            inner: self.inner.clone(),
            _marker: PhantomData::default(),
        }
    }
}

impl<S, INFO: AuthInfo> Service<Request> for HasPermissionService<S, INFO>
where
    S: Service<Request, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: IntoResponse,
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
        let cloned = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, cloned);
        let perm = self.permission.clone();

        Box::pin(async move {
            let (request, info) = match get_auth_info::<INFO>(request).await {
                Ok(x) => x,
                Err(e) => return Ok(e.into_response()),
            };

            if !info.has_permission(&perm) {
                return Ok(AuthError::MissingPermission(perm).into_response());
            }

            inner.call(request).await
        })
    }
}

/// Generate a middleware layer that checks if the user's AuthInfo has a specific predicate
pub fn has_auth_predicate<INFO, F>(
    message: impl Into<Cow<'static, str>>,
    f: F,
) -> HasPredicateLayer<INFO, F>
where
    INFO: AuthInfo,
    F: Fn(&INFO) -> bool + Clone,
{
    HasPredicateLayer {
        message: message.into(),
        f,
        _marker: PhantomData::default(),
    }
}

/// The middleware layer for checking an auth predicate
pub struct HasPredicateLayer<INFO, F>
where
    INFO: AuthInfo,
    F: Fn(&INFO) -> bool + Clone,
{
    message: Cow<'static, str>,
    f: F,
    _marker: PhantomData<INFO>,
}

impl<INFO, F> Clone for HasPredicateLayer<INFO, F>
where
    INFO: AuthInfo,
    F: Fn(&INFO) -> bool + Clone,
{
    fn clone(&self) -> Self {
        Self {
            message: self.message.clone(),
            f: self.f.clone(),
            _marker: PhantomData::default(),
        }
    }
}

impl<S, INFO, F> Layer<S> for HasPredicateLayer<INFO, F>
where
    INFO: AuthInfo,
    F: Fn(&INFO) -> bool + Clone,
{
    type Service = HasPredicateService<S, INFO, F>;

    fn layer(&self, inner: S) -> Self::Service {
        HasPredicateService {
            inner,
            message: self.message.clone(),
            f: self.f.clone(),
            _marker: PhantomData::default(),
        }
    }
}

/// The middleware service for checking an auth predicate
pub struct HasPredicateService<S, INFO, F>
where
    INFO: AuthInfo,
    F: Fn(&INFO) -> bool + Clone,
{
    message: Cow<'static, str>,
    f: F,
    inner: S,
    _marker: PhantomData<INFO>,
}

impl<S, INFO, F> Clone for HasPredicateService<S, INFO, F>
where
    INFO: AuthInfo,
    F: Fn(&INFO) -> bool + Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            message: self.message.clone(),
            f: self.f.clone(),
            inner: self.inner.clone(),
            _marker: self._marker.clone(),
        }
    }
}

impl<S, INFO, F> Service<Request> for HasPredicateService<S, INFO, F>
where
    INFO: AuthInfo,
    F: Fn(&INFO) -> bool + Clone + Send + 'static,
    S: Service<Request, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: IntoResponse,
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
        let cloned = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, cloned);
        let message = self.message.clone();
        let f = self.f.clone();

        Box::pin(async move {
            let (request, info) = match get_auth_info::<INFO>(request).await {
                Ok(x) => x,
                Err(e) => return Ok(e.into_response()),
            };

            if !f(&info) {
                return Ok(AuthError::FailedPredicate(message).into_response());
            }

            inner.call(request).await
        })
    }
}
