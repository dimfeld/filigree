use sqlx::PgExecutor;

use crate::auth::UserId;

/// Add a new user email login mapping. If `preverfied` is false, the verification token will be
/// returned.
pub async fn add_user_email_login(
    tx: impl PgExecutor<'_>,
    user_id: UserId,
    email: String,
    preverified: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO email_logins (user_id, email, verified)
       VALUES ($1, $2, $3)",
        user_id.as_uuid(),
        email,
        preverified,
    )
    .execute(tx)
    .await?;

    Ok(())
}
