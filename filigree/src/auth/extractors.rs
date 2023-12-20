use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};

use super::{AuthError, AuthInfo};

/// Extract authentication info from the Request, or return an error if the user is not valid.
pub struct Authed<T: AuthInfo>(T);

#[async_trait]
impl<S, T: AuthInfo + 'static> FromRequestParts<S> for Authed<T>
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth = parts
            .extensions
            .get::<T>()
            .cloned()
            .ok_or(AuthError::Unauthenticated)?;
        auth.check_valid()?;
        Ok(Authed(auth))
    }
}
