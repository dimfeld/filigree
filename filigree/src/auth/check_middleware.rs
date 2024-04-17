use std::{borrow::Cow, marker::PhantomData};

use axum::{extract::Request, response::IntoResponse};
use futures::future::BoxFuture;
use tower::{Layer, Service};

use super::{get_auth_info, AuthError, AuthInfo};

/// Check if the user has a particular set of permissions. This mostly exists to allow
/// [has_permission], [has_any_permission], and [has_all_permissions] to share the same
/// service and layer code.
pub trait PermissionChecker<INFO: AuthInfo>: Clone + Send + Sync + 'static {
    /// Perform the check, and return a missing permission if the check fails.
    fn check(&self, info: &INFO) -> Result<(), Cow<'static, str>>;
}

/// Check that the user has a particular permission.
#[derive(Debug)]
pub struct CheckOnePermission<INFO: AuthInfo> {
    perm: Cow<'static, str>,
    _marker: PhantomData<INFO>,
}

impl<INFO: AuthInfo> PermissionChecker<INFO> for CheckOnePermission<INFO> {
    fn check(&self, info: &INFO) -> Result<(), Cow<'static, str>> {
        match info.has_permission(&self.perm) {
            true => Ok(()),
            false => Err(self.perm.clone()),
        }
    }
}

impl<INFO: AuthInfo> Clone for CheckOnePermission<INFO> {
    fn clone(&self) -> Self {
        Self {
            perm: self.perm.clone(),
            _marker: PhantomData,
        }
    }
}

/// Check that the user has all of the given permissions.
#[derive(Debug)]
pub struct CheckAllPermissions<INFO: AuthInfo> {
    perms: Vec<Cow<'static, str>>,
    _marker: PhantomData<INFO>,
}

impl<INFO: AuthInfo> PermissionChecker<INFO> for CheckAllPermissions<INFO> {
    fn check(&self, info: &INFO) -> Result<(), Cow<'static, str>> {
        for perm in &self.perms {
            if !info.has_permission(perm) {
                return Err(perm.clone());
            }
        }

        Ok(())
    }
}

impl<INFO: AuthInfo> Clone for CheckAllPermissions<INFO> {
    fn clone(&self) -> Self {
        Self {
            perms: self.perms.clone(),
            _marker: PhantomData,
        }
    }
}

/// Check that the user has any of the given permissions.
#[derive(Debug)]
pub struct CheckAnyPermission<INFO: AuthInfo> {
    perms: Vec<Cow<'static, str>>,
    _marker: PhantomData<INFO>,
}

impl<INFO: AuthInfo> PermissionChecker<INFO> for CheckAnyPermission<INFO> {
    fn check(&self, info: &INFO) -> Result<(), Cow<'static, str>> {
        for perm in &self.perms {
            if info.has_permission(perm) {
                return Ok(());
            }
        }

        Err(self.perms[0].clone())
    }
}

impl<INFO: AuthInfo> Clone for CheckAnyPermission<INFO> {
    fn clone(&self) -> Self {
        Self {
            perms: self.perms.clone(),
            _marker: PhantomData,
        }
    }
}

/// Generate a middleware layer that checks if the user has a particular permission
pub fn has_permission<INFO: AuthInfo>(
    s: impl Into<Cow<'static, str>>,
) -> HasPermissionLayer<INFO, CheckOnePermission<INFO>> {
    HasPermissionLayer {
        checker: CheckOnePermission {
            perm: s.into(),
            _marker: PhantomData,
        },
        _marker: PhantomData,
    }
}

/// Generate a middleware layer that checks if the user has all of the given permissions
pub fn has_any_permission<INFO: AuthInfo>(
    perms: Vec<impl Into<Cow<'static, str>>>,
) -> HasPermissionLayer<INFO, CheckAnyPermission<INFO>> {
    if perms.is_empty() {
        panic!("`has_any_permission` requires at least one permission");
    }

    HasPermissionLayer {
        checker: CheckAnyPermission {
            perms: perms.into_iter().map(|s| s.into()).collect(),
            _marker: PhantomData,
        },
        _marker: PhantomData,
    }
}
/// Generate a middleware layer that checks if the user has a particular permission
pub fn has_all_permissions<INFO: AuthInfo>(
    perms: Vec<impl Into<Cow<'static, str>>>,
) -> HasPermissionLayer<INFO, CheckAllPermissions<INFO>> {
    if perms.is_empty() {
        panic!("`has_all_permission` requires at least one permission");
    }

    HasPermissionLayer {
        checker: CheckAllPermissions {
            perms: perms.into_iter().map(|s| s.into()).collect(),
            _marker: PhantomData,
        },
        _marker: PhantomData,
    }
}

/// Middleware layer that checks if the user has a particular permission
pub struct HasPermissionLayer<INFO: AuthInfo, CHECKER: PermissionChecker<INFO>> {
    checker: CHECKER,
    _marker: PhantomData<INFO>,
}

impl<INFO: AuthInfo, CHECKER: PermissionChecker<INFO>> Clone for HasPermissionLayer<INFO, CHECKER> {
    fn clone(&self) -> Self {
        Self {
            checker: self.checker.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, INFO: AuthInfo, CHECKER: PermissionChecker<INFO>> Layer<S>
    for HasPermissionLayer<INFO, CHECKER>
{
    type Service = HasPermissionService<S, INFO, CHECKER>;

    fn layer(&self, inner: S) -> Self::Service {
        HasPermissionService {
            checker: self.checker.clone(),
            inner,
            _marker: PhantomData,
        }
    }
}

/// The middleware service for checking if the user has a particular permission
pub struct HasPermissionService<S, INFO: AuthInfo, CHECKER: PermissionChecker<INFO>> {
    checker: CHECKER,
    inner: S,
    _marker: PhantomData<INFO>,
}

impl<S: Clone, INFO: AuthInfo, CHECKER: PermissionChecker<INFO>> Clone
    for HasPermissionService<S, INFO, CHECKER>
{
    fn clone(&self) -> Self {
        Self {
            checker: self.checker.clone(),
            inner: self.inner.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, INFO: AuthInfo, CHECKER: PermissionChecker<INFO>> Service<Request>
    for HasPermissionService<S, INFO, CHECKER>
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
        let checker = self.checker.clone();

        Box::pin(async move {
            let (request, info) = match get_auth_info::<INFO>(request).await {
                Ok(x) => x,
                Err(e) => return Ok(e.into_response()),
            };

            if let Err(perm) = checker.check(&info) {
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
        _marker: PhantomData,
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
            _marker: PhantomData,
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
            _marker: PhantomData,
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

/// Generate a middleware layer that disallows anonymous fallback users.
pub fn not_anonymous<INFO>() -> NotAnonymousLayer<INFO>
where
    INFO: AuthInfo,
{
    NotAnonymousLayer {
        _marker: PhantomData,
    }
}

/// The middleware layer for disallowing anonymous fallback users
pub struct NotAnonymousLayer<INFO>
where
    INFO: AuthInfo,
{
    _marker: PhantomData<INFO>,
}

impl<INFO> Clone for NotAnonymousLayer<INFO>
where
    INFO: AuthInfo,
{
    fn clone(&self) -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<S, INFO> Layer<S> for NotAnonymousLayer<INFO>
where
    INFO: AuthInfo,
{
    type Service = NotAnonymousService<S, INFO>;

    fn layer(&self, inner: S) -> Self::Service {
        NotAnonymousService {
            inner,
            _marker: PhantomData,
        }
    }
}

/// The middleware service for disallowing anonymous fallback users
pub struct NotAnonymousService<S, INFO>
where
    INFO: AuthInfo,
{
    inner: S,
    _marker: PhantomData<INFO>,
}

impl<S, INFO> Clone for NotAnonymousService<S, INFO>
where
    INFO: AuthInfo,
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _marker: self._marker.clone(),
        }
    }
}

impl<S, INFO> Service<Request> for NotAnonymousService<S, INFO>
where
    INFO: AuthInfo,
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

        Box::pin(async move {
            let (request, info) = match get_auth_info::<INFO>(request).await {
                Ok(x) => x,
                Err(e) => return Ok(e.into_response()),
            };

            if info.is_anonymous() {
                return Ok(AuthError::Unauthenticated.into_response());
            }

            inner.call(request).await
        })
    }
}
