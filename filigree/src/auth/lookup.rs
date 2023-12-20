use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, Request},
    http::{header::AUTHORIZATION, request::Parts},
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use error_stack::Report;
use sqlx::PgPool;
use tokio::sync::Mutex;

use super::{
    sessions::{get_session_cookie, SessionKey},
    AuthError, AuthInfo,
};

/// Options to create an AuthLookup object
pub struct AuthLookupOptions {
    /// The database pool
    pub pool: PgPool,
    /// The query to fetch the AuthInfo from a session. If you used the filigree CLI scaffolding,
    /// this should be `include_str!("src/auth/fetch_session.sql")`
    pub session_fetch_query: &'static str,
    /// The query to fetch the AuthInfo from a session. If you used the filigree CLI scaffolding,
    /// this should be `include_str!("src/auth/fetch_api_key.sql")`
    pub api_key_fetch_query: &'static str,
}

pub struct AuthLookup<T: AuthInfo> {
    info: Mutex<Option<Result<T, AuthError>>>,
    pool: PgPool,
    session_fetch_query: &'static str,
    api_key_fetch_query: &'static str,
}

impl<T: AuthInfo> AuthLookup<T> {
    pub fn new(options: AuthLookupOptions) -> Self {
        Self {
            info: Mutex::new(None),
            pool: options.pool,
            session_fetch_query: options.session_fetch_query,
            api_key_fetch_query: options.api_key_fetch_query,
        }
    }

    async fn get_info_from_api_key(&self, key: &str) -> Result<T, AuthError> {
        sqlx::query_as::<_, T>(self.api_key_fetch_query)
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AuthError::Db(Arc::new(e)))?
            .ok_or(AuthError::InvalidApiKey)
    }

    async fn get_info_from_session(&self, key: &SessionKey) -> Result<T, AuthError> {
        sqlx::query_as::<_, T>(self.api_key_fetch_query)
            .bind(&key.session_id)
            .bind(&key.hash)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AuthError::Db(Arc::new(e)))?
            .ok_or(AuthError::Unauthenticated)
    }

    async fn fetch_auth_info<S: Send + Sync>(
        &self,
        request: &mut Parts,
        _state: &S,
    ) -> Result<T, AuthError> {
        // Look for API key
        let bearer: Option<TypedHeader<Authorization<Bearer>>> =
            TypedHeader::from_request_parts(request, _state).await.ok();

        if let Some(bearer) = bearer {
            let key = bearer.0.token();
            return self.get_info_from_api_key(key).await;
        }

        let session_key = get_session_cookie(request);
        if let Some(session_key) = session_key {
            return self.get_info_from_session(&session_key).await;
        }

        Err(AuthError::Unauthenticated)
    }

    pub async fn get_auth_info<S: Send + Sync>(
        &self,
        request: &mut Parts,
        state: &S,
    ) -> Result<T, AuthError> {
        let mut info = self.info.lock().await;
        if let Some(info) = info.as_ref() {
            return info.clone();
        }

        let fetched = self.fetch_auth_info(request, state).await;
        *info = Some(fetched.clone());
        drop(info);

        fetched
    }
}
