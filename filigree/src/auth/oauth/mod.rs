use std::sync::Arc;

use axum::response::{IntoResponse, Redirect};
use error_stack::Report;
use hyper::StatusCode;
use oauth2::{reqwest::async_http_client, AuthorizationCode, TokenResponse};
use thiserror::Error;
use tower_cookies::Cookies;

use self::providers::{OAuthProvider, OAuthUserDetails};
use super::{AuthError, SessionError, UserId};
use crate::{
    errors::{ForceObfuscate, HttpError},
    server::FiligreeState,
};

/// OAuth provider implementations
pub mod providers;

#[derive(Error, Debug)]
pub enum OAuthError {
    #[error("Login session not found")]
    SessionNotFound,
    #[error("Login session expired")]
    SessionExpired,
    #[error("Failed to exchange code")]
    ExchangeError(
        #[from] oauth2::basic::BasicRequestTokenError<oauth2::reqwest::Error<reqwest::Error>>,
    ),
    #[error("Database error")]
    Db(Arc<sqlx::Error>),
    #[error("Failed to fetch user details")]
    FetchUserDetails(reqwest::Error),
    #[error("Session backend error")]
    SessionBackend(Report<SessionError>),
}

impl From<sqlx::Error> for OAuthError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(Arc::new(e))
    }
}

impl HttpError for OAuthError {
    type Detail = Option<String>;

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Db(_) | Self::ExchangeError(_) | Self::FetchUserDetails(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            _ => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_detail(&self) -> Self::Detail {
        match self {
            Self::ExchangeError(e) => Some(e.to_string()),
            Self::FetchUserDetails(e) => Some(e.to_string()),
            Self::SessionBackend(e) => Some(e.to_string()),
            Self::Db(e) => Some(e.to_string()),
            _ => None,
        }
    }

    fn obfuscate(&self) -> Option<ForceObfuscate> {
        if self.status_code() == StatusCode::UNAUTHORIZED {
            Some(ForceObfuscate {
                kind: "unauthenticated".into(),
                message: "Unauthenticated".into(),
            })
        } else {
            None
        }
    }

    fn error_kind(&self) -> &'static str {
        match self {
            Self::Db(_) => "db",
            Self::SessionExpired => "oauth_session_expired",
            Self::SessionNotFound => "oauth_session_not_found",
            Self::SessionBackend(_) => "session_backend_error",
            Self::FetchUserDetails(_) => "fetch_oauth_user_details",
            Self::ExchangeError(_) => "oauth_exchange_error",
        }
    }
}

/// A successful OAuth login response
pub struct OAuthLoginResponse {
    /// Information about the user from the OAuth provider
    pub user_details: OAuthUserDetails,
    /// An existing user ID to link to, if the user's email isn't aleady known.
    pub link_to_user: Option<UserId>,
    /// The URL to redirect the user to after login
    pub redirect_to: Option<String>,
}

/// Handle a successful login with an OAuth provider, which should have returned a token that can
/// be exchanged for an access token.
pub async fn handle_login_code(
    state: &FiligreeState,
    provider: Box<dyn OAuthProvider>,
    state_code: String,
    authorization_code: String,
) -> Result<OAuthLoginResponse, OAuthError> {
    let result = sqlx::query!(
        "DELETE FROM oauth_authorization_sessions
        WHERE key = $1
        RETURNING provider, expires_at, add_to_user_id, redirect_to",
        &state_code,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(OAuthError::SessionNotFound)?;

    if result.expires_at < chrono::Utc::now() || result.provider != provider.name() {
        return Err(OAuthError::SessionExpired);
    }

    let token_response = provider
        .client()
        .exchange_code(AuthorizationCode::new(authorization_code))
        .request_async(async_http_client)
        .await
        .map_err(OAuthError::ExchangeError)?;
    let access_token = token_response.access_token();

    let user_info = provider
        .fetch_user_details(access_token.secret())
        .await
        .map_err(OAuthError::FetchUserDetails)?;

    Ok(OAuthLoginResponse {
        link_to_user: result.add_to_user_id.map(UserId::from_uuid),
        redirect_to: result.redirect_to,
        user_details: user_info,
    })

    // For caller:
    // TODO Add/update oauth_logins
    // If there's an existing user_id and we haven't seen this email before, link this one into it.
    // TODO Add email address to account if not already present
    // Create a new user if we need to
    // Create session and cookie
    // Redirect to redirect_to, if it's set
}

/// Store state for an OAuth provider and redirects the user to the provider
pub async fn start_oauth_login(
    state: &FiligreeState,
    provider: Box<dyn OAuthProvider>,
    link_account: Option<UserId>,
    redirect_to: Option<String>,
) -> Result<impl IntoResponse, OAuthError> {
    let (url, csrf_token) = provider.authorize_url();

    sqlx::query!(
        "INSERT INTO oauth_authorization_sessions
            (key, provider, add_to_user_id, redirect_to, expires_at)
            VALUES
            ($1, $2, $3, $4, now() + '10 minutes'::interval)",
        csrf_token.secret(),
        provider.name(),
        link_account.map(|u| u.0),
        redirect_to
    )
    .execute(&state.db)
    .await?;

    Ok(Redirect::to(url.as_str()))
}
