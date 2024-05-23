use error_stack::{Report, ResultExt};
use tower_cookies::Cookies;
use uuid::Uuid;

use super::{AuthError, UserId};
use crate::server::FiligreeState;

/// A successful result of creating a login token
#[derive(Debug)]
pub struct PasswordlessLoginRequestAnswer {
    /// The login token
    pub token: Uuid,
    /// If this token is for a new user or not.
    pub new_user: bool,
}

/// Generate a new passwordless login token.
pub async fn setup_passwordless_login(
    state: &FiligreeState,
    email: String,
) -> Result<PasswordlessLoginRequestAnswer, Report<AuthError>> {
    let token = Uuid::new_v4();

    let found_email = {
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
        .change_context(AuthError::Db)?;

        result.rows_affected() > 0
    };

    if found_email {
        Ok(PasswordlessLoginRequestAnswer {
            token,
            new_user: false,
        })
    } else if state.new_user_flags.allow_public_signup {
        sqlx::query!(
            "INSERT INTO user_invites (email, organization_id, token, token_expires_at)
                VALUES ($1, NULL, $2, now() + interval '1 hour')
                ON CONFLICT(email, organization_id)
                DO UPDATE SET invite_sent_at = now(),
                    token = $2,
                    token_expires_at = now() + interval '1 hour'",
            email,
            token
        )
        .execute(&state.db)
        .await
        .change_context(AuthError::Db)?;

        Ok(PasswordlessLoginRequestAnswer {
            token,
            new_user: true,
        })
    } else {
        Err(Report::from(AuthError::UserNotFound))
    }
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
            passwordless_login_expires_at = null,
            verified = upd.verified OR
                (upd.passwordless_login_token = $2 AND upd.passwordless_login_expires_at > now())
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
    .change_context(AuthError::Db)?;

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

/// Accept a signup request. This only verifies the invite, and doesn't actually add the
/// user to the application.
pub async fn check_signup_request(
    state: &FiligreeState,
    email: &str,
    token: Uuid,
) -> Result<(), Report<AuthError>> {
    let result = sqlx::query!(
        "DELETE FROM user_invites
        WHERE email=$1 AND organization_id IS NULL
        RETURNING token, token_expires_at",
        email
    )
    .fetch_optional(&state.db)
    .await
    .change_context(AuthError::Db)?
    .ok_or(AuthError::InvalidToken)?;

    if result.token != token || result.token_expires_at < chrono::Utc::now() {
        return Err(Report::new(AuthError::InvalidToken));
    }

    Ok(())
}
