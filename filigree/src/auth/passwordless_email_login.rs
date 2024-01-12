use chrono::TimeZone;
use error_stack::{Report, ResultExt};
use tower_cookies::Cookies;
use uuid::Uuid;

use super::{AuthError, UserId};
use crate::server::FiligreeState;

/// Generate a new passwordless login token.
pub async fn setup_passwordless_login(
    state: &FiligreeState,
    email: String,
) -> Result<Uuid, Report<AuthError>> {
    let token = Uuid::new_v4();

    // TODO get the user name here too if we have it
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
        Err(AuthError::Unauthenticated)?;
    }

    Ok(token)
}

/// Given a token from an email, log in the user.
pub async fn perform_passwordless_login(
    state: &FiligreeState,
    cookies: &Cookies,
    email: String,
    token: Uuid,
) -> Result<(), Report<AuthError>> {
    // Get the token, and unconditionally clear it.
    let result = sqlx::query!(
        r##"
        UPDATE email_logins upd
        SET passwordless_login_token = null,
            passwordless_login_expires_at = null
        -- self-join since it lets us get the token even while we clear it in the UPDATE
        FROM email_logins old
        WHERE old.email = upd.email
            AND upd.email = $1
            AND upd.passwordless_login_token IS NOT NULL
        RETURNING old.user_id AS "user_id: UserId",
            (old.passwordless_login_token = $2 AND old.passwordless_login_expires_at > now()) AS valid
        "##,
        email,
        token
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AuthError::from)?;

    let user = result
        .as_ref()
        .filter(|r| r.valid.unwrap_or(false))
        .map(|r| r.user_id);

    let Some(user_id) = user else {
        return Err(Report::new(AuthError::InvalidToken));
    };

    state
        .session_backend
        .create_session(cookies, &user_id)
        .await
        .change_context(AuthError::SessionBackend)?;

    Ok(())
}
