use uuid::Uuid;

use super::ApiKey;
use crate::auth::{AuthError, OrganizationId, UserId};

/// Retrieve an API key
pub async fn get_api_key(
    pool: &sqlx::PgPool,
    api_key_id: &Uuid,
    hash: &[u8],
) -> Result<ApiKey, AuthError> {
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
                AND (expires_at IS NULL OR expires_at < now())"##,
        api_key_id,
        hash
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AuthError::InvalidApiKey)
}

/// List the API keys for a user
pub async fn list_api_keys(
    pool: &sqlx::PgPool,
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

/// Update an existing API key
pub async fn update_api_key(
    pool: &sqlx::PgPool,
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
            active = COALESCE($5, active),
            expires_at = COALESCE($6, expires_at)
        WHERE
            api_key_id = $1
            AND organization_id = $2
            AND user_id IS NOT DISTINCT FROM $3
        "##,
        api_key_id,
        organization_id.as_uuid(),
        user_id.as_ref().map(|id| id.as_uuid()),
        body.description,
        body.active,
        body.expires_at
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Set an API key enabled or disabled
pub async fn set_api_key_enabled(
    pool: &sqlx::PgPool,
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
    pool: &sqlx::PgPool,
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

/// Add a newly created API key into the database
pub async fn insert_api_key(
    pool: &sqlx::PgPool,
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
