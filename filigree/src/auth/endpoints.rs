use axum::{
    extract::{FromRef, State},
    response::IntoResponse,
    routing::Router,
    Json,
};
use serde::Deserialize;
use tower_cookies::Cookies;

use super::{
    password::{login_with_password, EmailAndPassword},
    AuthError, SessionBackend, SessionError,
};
use crate::{errors::WrapReport, Message};

/// Try to log in with a username and password, and create a session if successful.
async fn password_login(
    State(session): State<SessionBackend>,
    cookies: Cookies,
    Json(body): Json<EmailAndPassword>,
) -> Result<impl IntoResponse, WrapReport<AuthError>> {
    login_with_password(&session, &cookies, body).await?;

    Ok(Json(Message::new("Logged in")))
}

/// Remove the current user's session
async fn logout(
    State(session): State<SessionBackend>,
    cookies: Cookies,
) -> Result<impl IntoResponse, WrapReport<SessionError>> {
    session.delete_session(&cookies).await?;

    Ok(Json(Message::new("Logged out")))
}

/// Create routes for logging in and logging out
pub fn create_routes<T>() -> Router<T>
where
    SessionBackend: FromRef<T> + Clone,
    T: Send + Sync + Clone + 'static,
{
    Router::new()
        .route("/login", axum::routing::post(password_login))
        .route("/logout", axum::routing::post(logout))
}
