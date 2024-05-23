use std::sync::Arc;

use axum::{
    extract::{FromRef, State},
    response::IntoResponse,
    routing::Router,
};
use axum_jsonschema::Json;
use error_stack::ResultExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tower_cookies::Cookies;
use uuid::Uuid;

use super::{
    password::{login_with_password, EmailAndPassword},
    AuthError, SessionError,
};
use crate::{errors::WrapReport, extract::FormOrJson, server::FiligreeState, Message};

/// Try to log in with a username and password, and create a session if successful.
async fn password_login(
    State(state): State<Arc<FiligreeState>>,
    cookies: Cookies,
    FormOrJson(body): FormOrJson<EmailAndPassword>,
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

/// Request a password reset.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct UpdatePasswordRequest {
    /// The email for which the password is being reset
    #[validate(email)]
    pub email: String,
    /// The reset password token.
    pub token: Uuid,
    /// The new password to set
    #[validate(length(min = 1))]
    pub password: String,
    /// Another copy of the new password, to ensure that it's correct
    #[validate(length(min = 1))]
    pub confirm: String,
}

async fn update_password(
    State(state): State<Arc<FiligreeState>>,
    FormOrJson(request): FormOrJson<UpdatePasswordRequest>,
) -> Result<(), WrapReport<AuthError>> {
    if request.password != request.confirm {
        return Err(WrapReport::from(AuthError::PasswordConfirmMismatch));
    }

    let hashed = super::password::new_hash(request.password).await?;

    let result = sqlx::query!(
        "WITH sel AS (
            SELECT user_id, (reset_token IS NOT DISTINCT FROM $2 AND reset_expires_at > now()) AS matches
            FROM email_logins
            WHERE email = $1
        ),
        upd_el AS (
            -- Always clear the token
            UPDATE email_logins
            SET reset_token = null, reset_expires_at = null
            WHERE email = $1 AND reset_token IS NOT NULL
        )
        UPDATE users
        SET password_hash = $3
        FROM sel
        WHERE users.id = sel.user_id AND sel.matches",
        request.email,
        request.token,
        hashed.0,
    )
    .execute(&state.db)
    .await
    .change_context(AuthError::Db)?;

    if result.rows_affected() == 0 {
        return Err(WrapReport::from(AuthError::InvalidToken));
    }

    Ok(())
}

/// Create routes for logging in and logging out
pub fn create_routes<T>() -> Router<T>
where
    Arc<FiligreeState>: FromRef<T> + Clone,
    T: Send + Sync + Clone + 'static,
{
    Router::new()
        .route(
            "/auth/update_password",
            axum::routing::post(update_password),
        )
        .route("/auth/login", axum::routing::post(password_login))
        .route("/auth/logout", axum::routing::post(logout))
}
