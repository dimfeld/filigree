use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Request},
    http::request::Parts,
};

use super::{lookup::AuthLookup, AuthError, AuthInfo};

/// Extract authentication info from the Request, or return an error if the user is not valid.
pub struct Authed<T: AuthInfo>(T);

#[async_trait]
impl<S, T: AuthInfo + 'static> FromRequestParts<S> for Authed<T>
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_info = get_auth_info_from_parts(parts).await?;
        Ok(Authed(auth_info))
    }
}

/// Extract the AuthInfo from Request [Parts]
pub async fn get_auth_info_from_parts<T: AuthInfo>(parts: &mut Parts) -> Result<T, AuthError> {
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
pub async fn get_auth_info<T: AuthInfo>(request: Request) -> Result<(Request, T), AuthError> {
    let (mut parts, body) = request.into_parts();
    let auth_info = get_auth_info_from_parts(&mut parts).await?;

    let request = Request::from_parts(parts, body);
    Ok((request, auth_info))
}
