use error_stack::{Report, ResultExt};
use sqlx::PgExecutor;

use crate::{
    models::{
        organization::OrganizationId,
        user::{User, UserCreatePayload, UserId},
    },
    Error,
};

pub async fn create_new_user(
    db: impl PgExecutor<'_>,
    user_id: UserId,
    organization_id: OrganizationId,
    payload: UserCreatePayload,
    password_plaintext: String,
) -> Result<User, Report<Error>> {
    let password_hash = filigree::auth::password::new_hash(password_plaintext)
        .await
        .change_context(Error::AuthSubsystem)?;

    create_new_user_with_prehashed_password(db, user_id, organization_id, payload, password_hash)
        .await
}

pub async fn create_new_user_with_prehashed_password(
    db: impl PgExecutor<'_>,
    user_id: UserId,
    organization_id: OrganizationId,
    payload: UserCreatePayload,
    password_hash: String,
) -> Result<User, Report<Error>> {
    let user = sqlx::query_file_as!(
        User,
        "src/users/create_user.sql",
        user_id.as_uuid(),
        organization_id.as_uuid(),
        password_hash,
        &payload.name,
        &payload.email,
        &payload.verified,
    )
    .fetch_one(db)
    .await
    .change_context(Error::Db)?;

    Ok(user)
}
