use error_stack::{Report, ResultExt};
use sqlx::{postgres::PgHasArrayType, query, PgConnection, PgExecutor};
use uuid::Uuid;

// TODO this can go in the main filigree crate
use crate::{
    models::{organization::OrganizationId, role::RoleId, user::UserId},
    Error,
};

pub async fn add_roles_to_user(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    user_id: UserId,
    role_ids: &[RoleId],
) -> Result<(), Report<Error>> {
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
    .await
    .change_context(Error::Db)?;

    Ok(())
}

pub async fn remove_roles_from_user(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    user_id: UserId,
    role_ids: &[RoleId],
) -> Result<(), Report<Error>> {
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
    .await
    .change_context(Error::Db)?;

    Ok(())
}

pub async fn add_permissions_to_role(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    role_id: impl AsRef<Uuid>,
    permissions: &[String],
) -> Result<(), Report<Error>> {
    query!(
        r##"
        INSERT INTO permissions (organization_id, actor_id, permission)
        (
          SELECT $1, $2, permission FROM UNNEST($3::text[]) permission
        )
        ON CONFLICT DO NOTHING
        "##,
        organization_id.as_uuid(),
        role_id.as_ref(),
        permissions
    )
    .execute(db)
    .await
    .change_context(Error::Db)?;

    Ok(())
}

pub async fn remove_permissions_from_role(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    role_id: RoleId,
    permissions: &[String],
) -> Result<(), Report<Error>> {
    query!(
        r##"
        DELETE FROM permissions
            WHERE
                organization_id = $1
                AND actor_id = $2
                AND permission = ANY($3)
        "##,
        organization_id.as_uuid(),
        role_id.as_uuid(),
        permissions
    )
    .execute(db)
    .await
    .change_context(Error::Db)?;

    Ok(())
}
