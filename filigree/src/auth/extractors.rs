use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Request},
    http::request::Parts,
};
use error_stack::Report;

use super::{lookup::AuthLookup, AuthError, AuthInfo};
use crate::errors::WrapReport;

/// Extract authentication info from the Request, or return an error if the user is not valid.
pub struct Authed<T: AuthInfo>(Arc<T>);

impl<T: AuthInfo> Authed<T> {
    /// Create a new `Authed` when you already have the `AuthInfo`
    pub fn new(auth_info: Arc<T>) -> Self {
        Self(auth_info)
    }
}

impl<T> Deref for Authed<T>
where
    T: AuthInfo,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<S, T: AuthInfo + 'static> FromRequestParts<S> for Authed<T>
where
    S: Send + Sync,
{
    type Rejection = WrapReport<AuthError>;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_info = get_auth_info_from_parts(parts).await?;
        Ok(Authed(auth_info))
    }
}

/// Extract the AuthInfo from Request [Parts]
pub async fn get_auth_info_from_parts<T: AuthInfo>(
    parts: &mut Parts,
) -> Result<Arc<T>, Report<AuthError>> {
    let auth_lookup = parts
        .extensions
        .get::<Arc<AuthLookup<T>>>()
        .cloned()
        .ok_or(AuthError::Unauthenticated)?;
    let auth_info = auth_lookup.get_auth_info(parts).await?;
    auth_info.check_valid()?;
    Ok(auth_info)
}

/// Extract the AuthInfo from a [Request]
pub async fn get_auth_info<T: AuthInfo>(
    request: Request,
) -> Result<(Request, Arc<T>), Report<AuthError>> {
    let (mut parts, body) = request.into_parts();
    let auth_info = get_auth_info_from_parts(&mut parts).await?;

    let request = Request::from_parts(parts, body);
    Ok((request, auth_info))
}
