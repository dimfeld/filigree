use sqlx::PgExecutor;
use tracing::instrument;

use crate::auth::{OrganizationId, UserId};

/// Add a user to an organization. This does not assign any roles, so you should
/// usually call [add_roles_to_user] after this with the appropriate roles.
#[instrument(skip(db))]
pub async fn add_user_to_organization(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO organization_members
            (organization_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING",
        organization_id.as_uuid(),
        user_id.as_uuid()
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Toggle a user's active state within the organization. Inactive users can not log in to the
/// organization.
#[instrument(skip(db))]
pub async fn set_user_active(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    user_id: UserId,
    active: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE organization_members
            SET active = $3
            WHERE
                organization_id = $1
                AND user_id = $2",
        organization_id.as_uuid(),
        user_id.as_uuid(),
        active
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Completely remove a user from an organization. In most cases the user should be set to inactive
/// first, using [set_user_active], as this allows the user's name to show up in places where their
/// ID is referenced. Users who are removed from an organization are invisible to members of that
/// organization.
#[instrument(skip(db))]
pub async fn remove_user_from_organization(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM organization_members
            WHERE
                organization_id = $1
                AND user_id = $2",
        organization_id.as_uuid(),
        user_id.as_uuid()
    )
    .execute(db)
    .await?;

    Ok(())
}
