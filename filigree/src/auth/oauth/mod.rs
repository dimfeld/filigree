use std::sync::Arc;

use axum::response::{IntoResponse, Redirect};
use hyper::StatusCode;
use oauth2::{reqwest::async_http_client, AuthorizationCode, TokenResponse};

use self::providers::OAuthProvider;
use super::{AuthError, UserId};
use crate::server::FiligreeState;

/// OAuth provider implementations
pub mod providers;

/// Handle a successful login with an OAuth provider, which should have returned a token that can
/// be exchanged for an access token.
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
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AuthError::Db(Arc::new(e)))?
    .ok_or(AuthError::InvalidToken)?;

    if result.expires_at < chrono::Utc::now() || result.provider != provider.name() {
        return Err(AuthError::InvalidToken);
    }

    let token_response = provider
        .client()
        .exchange_code(AuthorizationCode::new(authorization_code))
        .request_async(async_http_client)
        .await
        .map_err(|_| AuthError::InvalidToken)?;
    let access_token = token_response.access_token();

    let user_info = provider.fetch_user_details(access_token.secret()).await;

    // TODO Get user info and update database
    // TODO Add/update oauth_logins

    // TODO Add email address to account if not already present
    // If there's an existing user_id and we haven't seen this email before, link this one into it.

    // Create session and cookie
    // Redirect to redirect_to, if it's set
    todo!();

    Ok(StatusCode::OK)
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
        provider.name(),
        link_account.map(|u| u.0),
        redirect_to
    )
    .execute(&state.db)
    .await
    .map_err(|e| AuthError::Db(Arc::new(e)))?;

    Ok(Redirect::to(url.as_str()))
}
