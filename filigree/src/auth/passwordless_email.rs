use std::sync::Arc;

use chrono::TimeZone;
use error_stack::{Report, ResultExt};
use tower_cookies::Cookies;
use uuid::Uuid;

use super::{AuthError, UserId};
use crate::{email::Email, server::FiligreeState};

/// Generate a new passwordless login token and send the email with the token to the user.
pub async fn request_passwordless_login(
    state: Arc<FiligreeState>,
    base_url: String,
    email: String,
) -> Result<(), Report<AuthError>> {
    let token = Uuid::new_v4();

    let result = sqlx::query!(
        "UPDATE email_logins
            SET passwordless_login_token = $2,
                passwordless_login_expires_at = now() + interval '1 hour'
            WHERE email = $1",
        email,
        &token
    )
    .execute(&state.db)
    .await
    .map_err(AuthError::from)?;

    let found_email = result.rows_affected() > 0;

    if !found_email {
        return Ok(());
    }

    state.email.send(Email::builder()
        .html(format!(r##"<a href="{base_url}/api/passwordless_login?email={email}&token={token}">Click here to login</a>"##))
        .to(email)
        .subject("Passwordless login")
        // TODO Real url here
        .tag("passwordless_login")
        .build()
    )
        .await
        .change_context(AuthError::EmailSendFailure)?;

    Ok(())
}

/// Given a token from an email, log in the user.
pub async fn perform_passwordless_login(
    state: Arc<FiligreeState>,
    cookies: &Cookies,
    email: String,
    token: Uuid,
) -> Result<(), Report<AuthError>> {
    let result = sqlx::query!(
        r##"UPDATE email_logins
            SET passwordless_login_token = null
            WHERE email = $1
                AND passwordless_login_token = $2
            RETURNING user_id AS "user_id: UserId", passwordless_login_expires_at"##,
        email,
        token
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AuthError::from)?
    .ok_or(AuthError::InvalidToken)?;

    let expiration = result
        .passwordless_login_expires_at
        .unwrap_or_else(|| chrono::Utc.timestamp_micros(0).unwrap());
    if expiration < chrono::Utc::now() {
        return Err(Report::new(AuthError::InvalidToken));
    }

    state
        .session_backend
        .create_session(cookies, &result.user_id)
        .await
        .change_context(AuthError::SessionBackend)?;

    Ok(())
}
