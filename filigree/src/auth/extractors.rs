use std::sync::Arc;

use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};

use super::{lookup::AuthLookup, AuthError, AuthInfo};

/// Extract authentication info from the Request, or return an error if the user is not valid.
pub struct Authed<T: AuthInfo>(T);

#[async_trait]
impl<S, T: AuthInfo + 'static> FromRequestParts<S> for Authed<T>
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = parts
            .extensions
            .get::<Arc<AuthLookup<T>>>()
            .cloned()
            .ok_or(AuthError::Unauthenticated)?;

        let auth_info = auth.get_auth_info(parts, state).await?;
        auth_info.check_valid()?;

        Ok(Authed(auth_info))
    }
}
