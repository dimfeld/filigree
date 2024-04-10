use axum::{
    extract::FromRequestParts,
    response::{Redirect, Response},
};
use http::{request::Parts, StatusCode};
use url::Url;

use crate::auth::AuthInfo;

pub struct WebAuthed(std::sync::Arc<AuthInfo>);

impl std::ops::Deref for WebAuthed {
    type Target = AuthInfo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for WebAuthed
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match filigree::auth::get_auth_info_from_parts(parts).await {
            Ok(auth_info) => Ok(WebAuthed(auth_info)),
            Err(e) => match e.status_code() {
                StatusCode::UNAUTHORIZED => {
                    let redirect_to = parts.uri().path_and_query().map(|p| p.as_str());

                    let mut login_url = Url::parse("/login").unwrap();
                    if let Some(r) = redirect_to {
                        login_url
                            .query_pairs_mut()
                            .append_pair("redirect_to", redirect_to.unwrap());
                    }

                    Err(Redirect::to(login_url.as_str()))
                }
                _ => Err(super::generic_error::generic_error_page(&e)),
            },
        }
    }
}
