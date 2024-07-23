use std::sync::Arc;

use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use error_stack::Report;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{
    sessions::{get_session_cookie, SessionKey},
    AuthError, AuthInfo, AuthQueries, UserFromRequestPartsValue, UserId,
};

/// When an anonymous users accesses the site, authenticate as this user instead.
/// This allows gives a proper context for anonymous users to interact with the site in a
/// meaningful way, if desired.
///
/// To define an anonymous user, insert this structure into the request extensions. The easiest way
/// to do this is through the [Extension] middleware, such as by adding
/// `.layer(Extension(FallbackAnonymousUser(user_id))` around the routes that you
/// want.
#[derive(Debug, Clone, Copy)]
pub struct FallbackAnonymousUser(pub UserId);

/// Functionality to fetch authorization info from the database given session cookies and Bearer tokens
pub struct AuthLookup<T: AuthInfo> {
    info: Mutex<Option<Arc<T>>>,
    // Erase the type so that we don't have to reference it everywhere such as
    // in the Authed extractor, which can become inconvenient. This also lets us potentially switch
    // auth providers without recompiling, which might be nice for certain open source projects.
    queries: Arc<dyn AuthQueries<AuthInfo = T>>,
}

impl<T: AuthInfo> AuthLookup<T> {
    /// Create a new AuthLookup
    pub fn new(queries: Arc<dyn AuthQueries<AuthInfo = T>>) -> Self {
        Self {
            info: Mutex::new(None),
            queries,
        }
    }

    async fn get_info_from_api_key(
        &self,
        key: Uuid,
        hash: Vec<u8>,
    ) -> Result<Arc<T>, Report<AuthError>> {
        self.queries
            .get_user_by_api_key(key, hash)
            .await?
            .map(Arc::new)
            .ok_or(Report::new(AuthError::InvalidApiKey))
    }

    async fn get_info_from_session(&self, key: &SessionKey) -> Result<Arc<T>, Report<AuthError>> {
        self.queries
            .get_user_by_session_id(key)
            .await?
            .map(Arc::new)
            .ok_or(Report::new(AuthError::Unauthenticated))
    }

    async fn fetch_auth_info(&self, request: &mut Parts) -> Result<Arc<T>, Report<AuthError>> {
        let general_result = self.queries.get_user_from_request_parts(request).await?;
        match general_result {
            UserFromRequestPartsValue::Found(info) => return Ok(Arc::new(info)),
            UserFromRequestPartsValue::NotFound => {
                return Err(Report::new(AuthError::Unauthenticated));
            }
            UserFromRequestPartsValue::NotImplemented => {
                // Just call the other functions
            }
        }

        // Look for API key
        let bearer: Option<TypedHeader<Authorization<Bearer>>> =
            TypedHeader::from_request_parts(request, &()).await.ok();

        if let Some(bearer) = bearer {
            let raw_key = bearer.0.token();
            let (key_id, hash) = super::api_key::decode_key(raw_key)?;
            return self.get_info_from_api_key(key_id, hash).await;
        }

        let session_key = get_session_cookie(request);
        if let Some(session_key) = session_key {
            match self.get_info_from_session(&session_key).await {
                Ok(info) => return Ok(info),
                Err(e) if e.current_context().is_unauthenticated() => {
                    let user = self.try_anonymous_user(request).await?;
                    return user.ok_or(Report::new(AuthError::Unauthenticated));
                }
                Err(e) => return Err(e),
            }
        }

        Err(Report::new(AuthError::Unauthenticated))
    }

    async fn try_anonymous_user(
        &self,
        request: &mut Parts,
    ) -> Result<Option<Arc<T>>, Report<AuthError>> {
        let Some(anon) = request.extensions.get::<FallbackAnonymousUser>() else {
            return Ok(None);
        };

        let info = self.queries.anonymous_user(anon.0.clone()).await?;
        Ok(info.map(Arc::new))
    }

    /// Return the authorization info, fetching it if it hasn't yet been fetched for this request.
    pub async fn get_auth_info(&self, request: &mut Parts) -> Result<Arc<T>, Report<AuthError>> {
        let mut info = self.info.lock().await;
        if let Some(info) = info.as_ref() {
            return Ok(info.clone());
        }

        let fetched = self.fetch_auth_info(request).await?;
        *info = Some(fetched.clone());

        Ok(fetched)
    }
}
