use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use error_stack::{Report, ResultExt};
use schemars::JsonSchema;
use serde::Deserialize;
use sqlx::PgPool;
use tower_cookies::Cookies;
use tracing::instrument;
use uuid::Uuid;

use super::{sessions::SessionBackend, AuthError, UserId};

/// Hash a password using a randomly-generated salt value
pub async fn new_hash(password: String) -> Result<String, AuthError> {
    let salt = uuid::Uuid::new_v4();
    hash_password(password, salt).await
}

#[instrument]
async fn hash_password(password: String, salt: Uuid) -> Result<String, AuthError> {
    let hash = tokio::task::spawn_blocking(move || {
        let saltstring = SaltString::encode_b64(salt.as_bytes())
            .map_err(|e| AuthError::PasswordHasherError(e.to_string()))?;

        let hash = Argon2::default()
            .hash_password(password.as_bytes(), saltstring.as_salt())
            .map_err(|e| AuthError::PasswordHasherError(e.to_string()))?;

        Ok::<_, AuthError>(hash.to_string())
    })
    .await
    .map_err(|e| AuthError::PasswordHasherError(e.to_string()))??;

    Ok(hash)
}

/// Verify that the given password matches the stored hash
pub async fn verify_password(password: String, hash_str: String) -> Result<(), AuthError> {
    tokio::task::spawn_blocking(move || {
        let hash = PasswordHash::new(&hash_str)
            .map_err(|e| AuthError::PasswordHasherError(e.to_string()))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .map_err(|_| AuthError::IncorrectPassword)
    })
    .await
    .map_err(|e| AuthError::PasswordHasherError(e.to_string()))??;

    Ok(())
}

/// An email and password to attempt login
#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmailAndPassword {
    #[validate(email)]
    email: String,
    #[validate(length(min = 1))]
    password: String,
}

/// Look up a user and verify the password, and check that the user is verified.
pub async fn lookup_user_from_email_and_password(
    db: &PgPool,
    email_and_password: EmailAndPassword,
) -> Result<UserId, Report<AuthError>> {
    if email_and_password.password.is_empty() {
        // This should really be caught earlier, but just make sure that nothing weird happens if
        // the user doesn't have a password (e.g. OAuth only) and someone tries to log in with an empty password.
        Err(AuthError::Unauthenticated)?;
    }

    let user_info = sqlx::query!(
        r#"SELECT user_id as "user_id: UserId", password_hash, email_logins.verified
        FROM email_logins
        JOIN users ON users.id = email_logins.user_id
        WHERE email_logins.email = $1"#,
        email_and_password.email
    )
    .fetch_optional(db)
    .await
    .map_err(AuthError::from)?
    .ok_or(AuthError::UserNotFound)?;

    let password_hash = user_info.password_hash.unwrap_or_default();

    verify_password(email_and_password.password, password_hash).await?;

    if !user_info.verified {
        return Err(Report::new(AuthError::NotVerified))?;
    }

    Ok(user_info.user_id)
}

/// Lookup a user based on the email/password, and create a new session.
/// This returns an error if the email is not found, the password is incorrect, or if the user is
/// not verified.
pub async fn login_with_password(
    session_backend: &SessionBackend,
    cookies: &Cookies,
    email_and_password: EmailAndPassword,
) -> Result<(), Report<AuthError>> {
    let user_id =
        lookup_user_from_email_and_password(&session_backend.db, email_and_password).await?;

    session_backend
        .create_session(&cookies, &user_id)
        .await
        .change_context(AuthError::SessionBackend)?;
    Ok(())
}

/// Create a password reset token
pub async fn create_reset_token(db: &sqlx::PgPool, email: &str) -> Result<Uuid, AuthError> {
    let token = Uuid::new_v4();

    let result = sqlx::query!(
        "UPDATE email_logins
        SET reset_token = $2,
            reset_expires_at = now() + '1 hour'::interval
        WHERE email = $1",
        email,
        &token
    )
    .execute(db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AuthError::Unauthenticated);
    }

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg_attr(
        not(any(feature = "test_slow", feature = "test_password")),
        ignore = "slow password test"
    )]
    async fn good_password() -> Result<(), AuthError> {
        let hash = new_hash("abcdef".into()).await?;
        verify_password("abcdef".to_string(), hash).await
    }

    #[tokio::test]
    #[cfg_attr(
        not(any(feature = "test_slow", feature = "test_password")),
        ignore = "slow password test"
    )]
    async fn bad_password() -> Result<(), AuthError> {
        let hash = new_hash("abcdef".into()).await?;
        verify_password("abcdefg".to_string(), hash)
            .await
            .expect_err("non-matching password");
        Ok(())
    }

    /// Test that the salt actually results in a different hash every time.
    #[tokio::test]
    #[cfg_attr(
        not(any(feature = "test_slow", feature = "test_password")),
        ignore = "slow password test"
    )]
    async fn unique_password_salt() {
        let p1 = new_hash("abc".into()).await.unwrap();
        let p2 = new_hash("abc".into()).await.unwrap();
        assert_ne!(p1, p2);
    }
}
