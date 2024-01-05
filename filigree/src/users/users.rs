use chrono::Utc;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::auth::UserId;

/// Add a new user email login mapping. If `preverfied` is false, the verification token will be
/// returned.
pub async fn add_user_email_login(
    tx: impl PgExecutor<'_>,
    user_id: UserId,
    email: String,
    preverified: bool,
) -> Result<Option<Uuid>, sqlx::Error> {
    let verify_token = (!preverified).then(Uuid::new_v4);
    let verify_expires_at = (!preverified).then(|| Utc::now() + chrono::Duration::hours(3));

    sqlx::query!(
        "INSERT INTO email_logins (user_id, email, verified, verify_token, verify_expires_at)
       VALUES ($1, $2, $3, $4, $5)",
        user_id.as_uuid(),
        email,
        preverified,
        verify_token,
        verify_expires_at
    )
    .execute(tx)
    .await?;

    Ok(verify_token)
}
