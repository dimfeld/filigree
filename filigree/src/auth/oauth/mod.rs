use std::sync::Arc;

use axum::response::{IntoResponse, Redirect};
use error_stack::{Report, ResultExt};
use hyper::StatusCode;
use oauth2::TokenResponse;
use sqlx::PgExecutor;
use thiserror::Error;
use tower_cookies::Cookies;

use self::providers::{AuthorizeUrl, OAuthProvider, OAuthUserDetails};
use super::UserId;
use crate::{
    errors::{ErrorKind, ForceObfuscate, HttpError, WrapReport},
    server::FiligreeState,
    users::users::CreateUserDetails,
};

/// OAuth provider implementations
pub mod providers;

/// An error returned from an OAuth2 interaction
#[derive(Error, Debug)]
pub enum OAuthError {
    /// The session referred to in the `state` parameter was not found. In production this will
    /// appear as a generic 401 error.
    #[error("Login session not found")]
    SessionNotFound,
    /// The session referred to in the `state` parameter was not found. In production this will
    /// appear as a generic 401 error.
    #[error("Login session expired")]
    SessionExpired,
    /// Attempted to exchange an OAuth login authorization token for an access token, but it didn't
    /// work.
    #[error("Failed to exchange code")]
    ExchangeError,
    /// The database returned an error
    #[error("Database error")]
    Db,
    /// Failure when trying to read the user details from the OAuth provider.
    #[error("Failed to fetch user details")]
    FetchUserDetails,
    /// Some error from the session backend
    #[error("Session backend error")]
    SessionBackend,
    /// Returned when user signups are disabled.
    #[error("Sorry, new signups are currently not allowed")]
    PublicSignupDisabled,
    #[error("Failed to create user")]
    UserCreation,
}

impl HttpError for OAuthError {
    type Detail = ();

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Db | Self::ExchangeError | Self::FetchUserDetails | Self::UserCreation => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::PublicSignupDisabled => StatusCode::FORBIDDEN,
            _ => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_detail(&self) -> Self::Detail {
        ()
    }

    fn obfuscate(&self) -> Option<ForceObfuscate> {
        if self.status_code() == StatusCode::UNAUTHORIZED {
            Some(ForceObfuscate::unauthenticated())
        } else {
            None
        }
    }

    fn error_kind(&self) -> &'static str {
        match self {
            Self::Db => ErrorKind::Database,
            Self::PublicSignupDisabled => ErrorKind::SignupDisabled,
            Self::SessionExpired => ErrorKind::OAuthSessionExpired,
            Self::SessionNotFound => ErrorKind::OAuthSessionNotFound,
            Self::SessionBackend => ErrorKind::SessionBackendError,
            Self::FetchUserDetails => ErrorKind::FetchOAuthUserDetails,
            Self::ExchangeError => ErrorKind::OAuthExchangeError,
            Self::UserCreation => ErrorKind::UserCreationError,
        }
        .as_str()
    }
}

/// Store state for an OAuth provider and redirects the user to the provider
pub async fn start_oauth_login(
    state: &FiligreeState,
    provider: Box<dyn OAuthProvider>,
    link_account: Option<UserId>,
    redirect_to: Option<String>,
) -> Result<impl IntoResponse, sqlx::Error> {
    let AuthorizeUrl {
        url,
        state: key,
        pkce_verifier,
    } = provider.authorize_url();

    sqlx::query!(
        "INSERT INTO oauth_authorization_sessions
            (key, provider, add_to_user_id, redirect_to, pkce_verifier, expires_at)
            VALUES
            ($1, $2, $3, $4, $5, now() + '10 minutes'::interval)",
        key.secret(),
        provider.name(),
        link_account.map(|u| u.0),
        redirect_to,
        pkce_verifier.as_ref().map(|p| p.secret()),
    )
    .execute(&state.db)
    .await?;

    Ok(Redirect::to(url.as_str()))
}

/// A successful OAuth login response
pub struct OAuthLoginResponse {
    /// The user ID for the user, if the user already exists. If this is None, you should
    /// create a new user.
    pub user_id: UserId,
    /// Information about the user from the OAuth provider
    pub user_details: OAuthUserDetails,
    /// The URL to redirect the user to after login
    pub redirect_to: Option<String>,
}

/// Link an OAuth login to a user
pub async fn add_oauth_login(
    db: impl PgExecutor<'_>,
    user_id: UserId,
    oauth_provider_name: &str,
    oauth_account_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO oauth_logins
            (user_id, oauth_provider, oauth_account_id)
            VALUES
            ($1, $2, $3)",
        user_id.0,
        oauth_provider_name,
        oauth_account_id
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Handle a successful login with an OAuth provider, which should have returned a token that can
/// be exchanged for an access token.
///
/// If the OAuth login session information indicates that this login should be linked to an
/// existing user, then this function will do that. Otherwise it creates a new user, if signups are
/// allowed.
///
/// This function returns a [OAuthLoginResponse] which has information about the user.
pub async fn handle_login_code(
    state: &FiligreeState,
    provider: Box<dyn OAuthProvider>,
    cookies: &Cookies,
    state_code: String,
    authorization_code: String,
) -> Result<OAuthLoginResponse, WrapReport<OAuthError>> {
    // TODO need to get either state code or pkce code
    let provider_name = provider.name();
    let oauth_login_session = sqlx::query!(
        "DELETE FROM oauth_authorization_sessions
        WHERE key = $1
        RETURNING provider, expires_at, pkce_verifier, add_to_user_id, redirect_to",
        &state_code,
    )
    .fetch_optional(&state.db)
    .await
    .change_context(OAuthError::Db)?
    .ok_or(OAuthError::SessionNotFound)?;

    if oauth_login_session.expires_at < chrono::Utc::now()
        || oauth_login_session.provider != provider_name
    {
        return Err(Report::new(OAuthError::SessionExpired).into());
    }

    let token_response = provider
        .fetch_access_token(
            authorization_code,
            oauth_login_session.pkce_verifier.unwrap_or_default(),
        )
        .await?;
    let access_token = token_response.access_token();

    let user_details = provider
        .fetch_user_details(state.http_client.clone(), access_token.secret())
        .await
        .change_context(OAuthError::FetchUserDetails)?;

    let mut tx = state.db.begin().await.change_context(OAuthError::Db)?;
    let existing_user = sqlx::query_scalar!(
        "SELECT user_id FROM oauth_logins
        WHERE oauth_provider = $1 AND oauth_account_id = $2",
        provider_name,
        &user_details.login_id
    )
    .fetch_optional(&mut *tx)
    .await
    .change_context(OAuthError::Db)?;

    let user_id = if let Some(existing_user) = existing_user {
        // This user already has an account.
        UserId::from(existing_user)
    } else if let Some(link_user_id) = oauth_login_session.add_to_user_id {
        let user_id = UserId::from(link_user_id);
        add_oauth_login(&mut *tx, user_id, provider_name, &user_details.login_id)
            .await
            .change_context(OAuthError::Db)?;
        user_id
    } else if !state.new_user_flags.allow_public_signup {
        return Err(Report::new(OAuthError::PublicSignupDisabled).into());
    } else {
        let create_user_details = CreateUserDetails {
            email: user_details.email.clone(),
            name: user_details.name.clone(),
            avatar_url: user_details.avatar_url.clone(),
            password_plaintext: None,
        };

        let user_id = state
            .user_creator
            .create_user(&mut tx, None, create_user_details)
            .await
            .change_context(OAuthError::UserCreation)?;
        add_oauth_login(&mut *tx, user_id, provider_name, &user_details.login_id)
            .await
            .change_context(OAuthError::Db)?;
        user_id
    };

    state
        .session_backend
        .create_session(cookies, &user_id)
        .await
        .change_context(OAuthError::SessionBackend)?;

    tx.commit().await.change_context(OAuthError::Db)?;

    // This user already has an account.
    Ok(OAuthLoginResponse {
        user_id,
        redirect_to: oauth_login_session.redirect_to,
        user_details,
    })
}
