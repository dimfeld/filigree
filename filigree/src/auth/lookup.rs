use std::sync::Arc;

use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{
    sessions::{get_session_cookie, SessionKey},
    AuthError, AuthInfo, AuthQueries,
};

/// Functionality to fetch authorization info from the database given session cookies and Bearer tokens
pub struct AuthLookup<T: AuthInfo> {
    info: Mutex<Option<Result<Arc<T>, AuthError>>>,
    // Erase the type so that we don't have to reference it everywhere such as
    // in the Authed extractor, which can become inconvenient.
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

    async fn get_info_from_api_key(&self, key: Uuid, hash: Vec<u8>) -> Result<Arc<T>, AuthError> {
        self.queries
            .get_user_by_api_key(key, hash)
            .await
            .map_err(AuthError::from)?
            .map(Arc::new)
            .ok_or(AuthError::InvalidApiKey)
    }

    async fn get_info_from_session(&self, key: &SessionKey) -> Result<Arc<T>, AuthError> {
        self.queries
            .get_user_by_session_id(key)
            .await
            .map_err(AuthError::from)?
            .map(Arc::new)
            .ok_or(AuthError::Unauthenticated)
    }

    async fn fetch_auth_info(&self, request: &mut Parts) -> Result<Arc<T>, AuthError> {
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
            return self.get_info_from_session(&session_key).await;
        }

        Err(AuthError::Unauthenticated)
    }

    /// Return the authorization info, fetching it if it hasn't yet been fetched for this request.
    pub async fn get_auth_info(&self, request: &mut Parts) -> Result<Arc<T>, AuthError> {
        let mut info = self.info.lock().await;
        if let Some(info) = info.as_ref() {
            return info.clone();
        }

        let fetched = self.fetch_auth_info(request).await;
        *info = Some(fetched.clone());

        fetched
    }
}
