use std::borrow::Cow;

use axum::{
    extract::FromRequestParts,
    response::{IntoResponse, Redirect, Response},
};
use axum_htmx::HxLocation;
use filigree::errors::HttpError;
use http::{request::Parts, StatusCode, Uri};

use crate::{
    auth::{AuthInfo, Authed},
    Error,
};

pub struct WebAuthed(pub Authed);

impl std::ops::Deref for WebAuthed {
    type Target = AuthInfo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Into<Authed> for WebAuthed {
    fn into(self) -> Authed {
        self.0
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
            Ok(auth_info) => Ok(WebAuthed(Authed::new(auth_info))),
            Err(e) => match e.status_code() {
                StatusCode::UNAUTHORIZED => {
                    let login_url = make_login_link(Some(&parts.uri));
                    Err((
                        HxLocation::from_uri(login_url.clone()),
                        Redirect::to(&login_url.to_string()),
                    )
                        .into_response())
                }
                _ => {
                    let e = Error::from(e);
                    Err(super::generic_error::generic_error_page(&e))
                }
            },
        }
    }
}

pub fn make_login_link(redirect_to: Option<&Uri>) -> Uri {
    if let Some(r) = redirect_to {
        let redirect_to = r
            .path_and_query()
            .map(|p| {
                Cow::Owned(
                    url::form_urlencoded::byte_serialize(p.as_str().as_bytes()).collect::<String>(),
                )
            })
            .unwrap_or(Cow::Borrowed("/"));
        format!("/login?redirect_to={redirect_to}")
            .parse::<Uri>()
            .unwrap_or_else(|_| "/login".parse().unwrap())
    } else {
        "/login".parse().unwrap()
    }
}
