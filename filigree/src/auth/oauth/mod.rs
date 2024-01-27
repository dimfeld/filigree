use std::sync::Arc;

use axum::response::{IntoResponse, Redirect};
use oauth2::{reqwest::async_http_client, AuthorizationCode, TokenResponse};

use self::providers::{OAuthProvider, OAuthUserDetails};
use super::{AuthError, Authed, UserId};
use crate::server::FiligreeState;

pub mod providers;

pub async fn handle_login_code(
    state: &FiligreeState,
    provider: Box<dyn OAuthProvider>,
    state_code: String,
    authorization_code: String,
) -> Result<impl IntoResponse, crate::auth::AuthError> {
    let result = sqlx::query!(
        "DELETE FROM oauth_authorization_sessions
        WHERE key = $1
        RETURNING provider, expires_at, add_to_user_id, redirect_to",
        &state_code,
        provider.name()
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AuthError::Db(Arc::new(e)))?
    .ok_or(AuthError::InvalidToken)?;

    if result.expires_at < chrono::Utc::now().naive_utc() || result.provider != provider.name() {
        return Err(AuthError::InvalidToken);
    }

    let access_token = provider
        .client()
        .exchange_code(AuthorizationCode::new(authorization_code))
        .request_async(async_http_client)
        .await
        .map_err(|_| AuthError::InvalidToken)?
        .access_token();

    // TODO Get user info and update database
    // TODO Add/update oauth_logins

    // TODO Add email address to account if not already present
    // If there's an existing user_id and we haven't seen this email before, link this one into it.
    let user_info = provider.fetch_user_details(access_token.secret()).await;

    // Create session and cookie
    // Redirect to redirect_to, if it's set
    todo!()
}

/// Stores state for an OAuth provider and redirects the user to the provider
pub async fn start_oauth_login(
    state: &FiligreeState,
    provider: Box<dyn OAuthProvider>,
    link_account: Option<UserId>,
    redirect_to: Option<String>,
) -> Result<impl IntoResponse, AuthError> {
    let (url, csrf_token) = provider.authorize_url();

    sqlx::query!(
        "INSERT INTO oauth_authorization_sessions
            (key, provider, add_to_user_id, redirect_to, expires_at)
            VALUES
            ($1, $2, $3, $4, now() + '10 minutes'::interval)",
        csrf_token.secret(),
        provider.name()
    )
    .execute(&state.db)
    .await
    .map_err(|e| AuthError::Db(Arc::new(e)))?;

    Ok(Redirect::to(url.as_str()))
}
