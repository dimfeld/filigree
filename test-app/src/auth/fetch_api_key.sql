WITH base_lookup AS (
  SELECT
    api_keys.user_id,
    -- API key always uses the organization the key was created with,
    -- regardless of the users currently-chosen org
    api_keys.organization_id,
    api_keys.inherits_user_permissions,
    om.active,
    users.verified
  FROM
    api_keys
    JOIN users ON users.id = api_keys.user_id
    JOIN organization_members om ON om.user_id = api_keys.user_id
      AND om.organization_id = api_keys.organization_id
  WHERE
    api_key_id = $1
    AND hash = $2
    -- API key must be enabled
    AND api_keys.active
    -- Disable API key if the user was removed from the org
    AND om.active
    -- API key must not be expired
    AND (expires_at IS NULL
      OR expires_at > now())
  LIMIT 1
),
role_lookup AS (
  SELECT
    role_id,
    organization_id
  FROM
    user_roles
    JOIN base_lookup USING (user_id, organization_id)
),
actor_ids AS (
  SELECT
    CASE WHEN inherits_user_permissions THEN
      user_id
    ELSE
      $1
    END AS actor_id,
    organization_id
  FROM
    base_lookup
UNION ALL
SELECT
  role_id AS actor_id,
  role_lookup.organization_id
FROM
  role_lookup
  CROSS JOIN base_lookup
  WHERE
    base_lookup.inherits_user_permissions
),
permissions AS (
  SELECT
    COALESCE(ARRAY_AGG(DISTINCT permission), ARRAY[]::text[]) AS permissions
  FROM
    permissions
    JOIN actor_ids USING (actor_id, organization_id))
SELECT
  bl.user_id AS "user_id!: crate::models::user::UserId",
  bl.organization_id AS "organization_id!: crate::models::organization::OrganizationId",
  bl.active,
  bl.verified,
  COALESCE((
    SELECT
      ARRAY_AGG(role_id) FILTER (WHERE role_id IS NOT NULL)
FROM role_lookup), ARRAY[]::uuid[]) AS "roles!: Vec<RoleId>",
  permissions AS "permissions!: Vec<String>"
FROM
  base_lookup bl
  CROSS JOIN permissions
