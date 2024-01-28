use sqlx::{query, PgExecutor};
use tracing::instrument;
use uuid::Uuid;

use crate::auth::{OrganizationId, RoleId, UserId};

/// Add roles to a user
#[instrument(skip(db))]
pub async fn add_roles_to_user(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    user_id: UserId,
    role_ids: &[RoleId],
) -> Result<(), sqlx::Error> {
    query!(
        r##"
        INSERT INTO user_roles (organization_id, user_id, role_id)
        (
          SELECT $1, $2, role_id FROM UNNEST($3::uuid[]) role_id
        )
        ON CONFLICT DO NOTHING
        "##,
        organization_id.as_uuid(),
        user_id.as_uuid(),
        role_ids as _
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Add the default role to a user, if one is set on the organization.
#[instrument(skip(db))]
pub async fn add_default_role_to_user(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    query!(
        r##"
        INSERT INTO user_roles (organization_id, user_id, role_id)
        (
            SELECT $1, $2, default_role as role_id
            FROM organizations
            WHERE id = $1 AND default_role IS NOT NULL
        )
        ON CONFLICT DO NOTHING
        "##,
        organization_id.as_uuid(),
        user_id.as_uuid(),
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Remove roles from a user
#[instrument(skip(db))]
pub async fn remove_roles_from_user(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    user_id: UserId,
    role_ids: &[RoleId],
) -> Result<(), sqlx::Error> {
    query!(
        r##"
        DELETE FROM user_roles
            WHERE
                organization_id = $1
                AND user_id = $2
                AND role_id = ANY($3)
        "##,
        organization_id.as_uuid(),
        user_id.as_uuid(),
        role_ids as _,
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Add org-wide permissions to a role or user
#[instrument(skip(db))]
pub async fn add_permissions_to_role(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    actor_id: impl AsRef<Uuid> + std::fmt::Debug,
    permissions: &[String],
) -> Result<(), sqlx::Error> {
    query!(
        r##"
        INSERT INTO permissions (organization_id, actor_id, permission)
        (
          SELECT $1, $2, permission FROM UNNEST($3::text[]) permission
        )
        ON CONFLICT DO NOTHING
        "##,
        organization_id.as_uuid(),
        actor_id.as_ref(),
        permissions
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Remove org-wide permissions from a role or user
#[instrument(skip(db))]
pub async fn remove_permissions_from_role(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    role_id: impl AsRef<Uuid> + std::fmt::Debug,
    permissions: &[String],
) -> Result<(), sqlx::Error> {
    query!(
        r##"
        DELETE FROM permissions
            WHERE
                organization_id = $1
                AND actor_id = $2
                AND permission = ANY($3)
        "##,
        organization_id.as_uuid(),
        role_id.as_ref(),
        permissions
    )
    .execute(db)
    .await?;

    Ok(())
}
