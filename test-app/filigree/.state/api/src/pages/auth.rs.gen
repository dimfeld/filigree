use axum::{
    extract::FromRequestParts,
    response::{IntoResponse, Redirect, Response},
};
use filigree::errors::HttpError;
use http::{request::Parts, StatusCode, Uri};
use url::Url;

use crate::{auth::AuthInfo, Error};

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
                    let login_url = make_login_link(Some(&parts.uri));
                    Err(Redirect::to(login_url.as_str()).into_response())
                }
                _ => {
                    let e = Error::from(e);
                    Err(super::generic_error::generic_error_page(&e))
                }
            },
        }
    }
}

pub fn make_login_link(redirect_to: Option<&Uri>) -> String {
    let mut login_url = Url::parse("/login").unwrap();
    if let Some(r) = redirect_to {
        let redirect_to = r.path_and_query().map(|p| p.as_str());
        login_url
            .query_pairs_mut()
            .append_pair("redirect_to", redirect_to.unwrap());
    }
    login_url.into()
}
