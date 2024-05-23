use error_stack::{Report, ResultExt};
use sqlx::PgExecutor;
use uuid::Uuid;

use super::ApiKey;
use crate::auth::{AuthError, OrganizationId, UserId};

/// Retrieve an API key, making sure that the hash matches and that the key is valid
/// In most cases you will prefer to call [lookup_api_key_from_bearer_token] instead, which
/// calls this after decoding the token.
pub async fn lookup_api_key_for_auth(
    pool: impl PgExecutor<'_>,
    api_key_id: &Uuid,
    hash: &[u8],
) -> Result<ApiKey, Report<AuthError>> {
    sqlx::query_as!(
        ApiKey,
        r##"SELECT api_key_id,
            organization_id,
            user_id AS "user_id: UserId",
            inherits_user_permissions,
            description,
            active,
            expires_at
            FROM api_keys
            WHERE
                api_key_id = $1
                AND hash = $2
                AND active
                AND expires_at > now()"##,
        api_key_id,
        hash
    )
    .fetch_optional(pool)
    .await
    .change_context(AuthError::Db)?
    .ok_or_else(|| Report::new(AuthError::InvalidApiKey))
}

/// List the API keys for a user
pub async fn list_api_keys(
    pool: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    user_id: Option<UserId>,
) -> Result<Vec<ApiKey>, sqlx::Error> {
    sqlx::query_as!(
        ApiKey,
        r##"SELECT api_key_id,
            organization_id,
            user_id AS "user_id: UserId",
            inherits_user_permissions,
            description,
            active,
            expires_at
            FROM api_keys
            WHERE
                organization_id = $1
                AND user_id IS NOT DISTINCT FROM $2"##,
        user_id.as_ref().map(|id| id.as_uuid()),
        organization_id.as_uuid()
    )
    .fetch_all(pool)
    .await
}

/// Add a newly created API key into the database
pub async fn add_api_key(
    pool: impl PgExecutor<'_>,
    key: &ApiKey,
    hash: &[u8],
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r##"INSERT INTO api_keys
            (api_key_id,
            organization_id,
            user_id,
            hash,
            inherits_user_permissions,
            description,
            active,
            expires_at)
            VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8)"##,
        key.api_key_id,
        key.organization_id.as_uuid(),
        key.user_id.as_ref().map(|id| id.as_uuid()),
        hash,
        key.inherits_user_permissions,
        key.description,
        key.active,
        key.expires_at,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Update an existing API key
pub async fn update_api_key(
    pool: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    user_id: Option<UserId>,
    api_key_id: &Uuid,
    body: &super::ApiKeyUpdateBody,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r##"
        UPDATE api_keys
        SET
            description = COALESCE($4, description),
            active = COALESCE($5, active)
        WHERE
            api_key_id = $1
            AND organization_id = $2
            AND user_id IS NOT DISTINCT FROM $3
        "##,
        api_key_id,
        organization_id.as_uuid(),
        user_id.as_ref().map(|id| id.as_uuid()),
        body.description,
        body.active
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Set an API key enabled or disabled
pub async fn set_api_key_enabled(
    pool: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    api_key_id: &Uuid,
    enabled: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r##"UPDATE api_keys
            SET active = $3
            WHERE api_key_id = $1
            AND organization_id = $2"##,
        api_key_id,
        organization_id.as_uuid(),
        enabled,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete an API key
pub async fn delete_api_key(
    pool: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    api_key_id: &Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r##"DELETE FROM api_keys
            WHERE
                api_key_id = $1
                AND organization_id = $2"##,
        api_key_id,
        organization_id.as_uuid(),
    )
    .execute(pool)
    .await?;

    Ok(())
}
