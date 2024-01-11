use std::sync::Arc;

use axum::{
    extract::{FromRef, State},
    response::IntoResponse,
    routing::Router,
    Json,
};
use tower_cookies::Cookies;

use super::{
    password::{login_with_password, EmailAndPassword},
    AuthError, SessionError,
};
use crate::{errors::WrapReport, server::FiligreeState, Message};

/// Try to log in with a username and password, and create a session if successful.
async fn password_login(
    State(state): State<Arc<FiligreeState>>,
    cookies: Cookies,
    Json(body): Json<EmailAndPassword>,
) -> Result<impl IntoResponse, WrapReport<AuthError>> {
    login_with_password(&state.session_backend, &cookies, body).await?;

    Ok(Json(Message::new("Logged in")))
}

/// Remove the current user's session
async fn logout(
    State(state): State<Arc<FiligreeState>>,
    cookies: Cookies,
) -> Result<impl IntoResponse, WrapReport<SessionError>> {
    state.session_backend.delete_session(&cookies).await?;

    Ok(Json(Message::new("Logged out")))
}

/// Create routes for logging in and logging out
pub fn create_routes<T>() -> Router<T>
where
    Arc<FiligreeState>: FromRef<T> + Clone,
    T: Send + Sync + Clone + 'static,
{
    Router::new()
        .route("/auth/login", axum::routing::post(password_login))
        .route("/auth/logout", axum::routing::post(logout))
}
