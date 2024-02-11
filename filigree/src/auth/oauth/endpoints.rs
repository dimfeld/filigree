use std::sync::Arc;

use axum::{
    extract::{FromRef, Path, Query, State},
    response::IntoResponse,
    routing, Router,
};
use hyper::StatusCode;
use serde::Deserialize;
use tower_cookies::Cookies;
use tracing::instrument;

use super::{handle_login_code, start_oauth_login, OAuthError};
use crate::{errors::WrapReport, server::FiligreeState};

/// Start an OAuth2 login
#[instrument(skip(state, cookies))]
pub async fn login(
    State(state): State<Arc<FiligreeState>>,
    cookies: Cookies,
    Path(provider_name): Path<String>,
) -> Result<impl IntoResponse, WrapReport<OAuthError>> {
    start_oauth_login(&state, &cookies, &provider_name, None, None)
        .await
        .map_err(WrapReport::from)
}

/// Query string for OAuth2 login callback
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    code: String,
    state: String,
}

/// OAuth2 Login callback
#[instrument(skip(state, cookies))]
pub async fn callback(
    State(state): State<Arc<FiligreeState>>,
    cookies: Cookies,
    Path(provider_name): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<impl IntoResponse, WrapReport<OAuthError>> {
    handle_login_code(&state, &cookies, &provider_name, query.state, query.code)
        .await
        .map_err(WrapReport::from)?;

    Ok(StatusCode::OK)
}

/// Create a default set of OAuth endpoints.
///
/// - /auth/oauth/:provider/login wraps `start_oauth_login`
/// - /auth/oauth/:provider/callback wraps `handle_login_code`
pub fn create_routes<T>() -> Router<T>
where
    Arc<FiligreeState>: FromRef<T> + Clone,
    T: Send + Sync + Clone + 'static,
{
    Router::new()
        .route("/auth/oauth/login/:provider", routing::get(login))
        .route(
            "/auth/oauth/login/:provider/callback",
            routing::get(callback),
        )
}
