WITH base_lookup AS (
  SELECT
    sess.user_id,
    users.organization_id,
    om.active,
    users.verified
  FROM
    user_sessions sess
    JOIN users ON sess.user_id = users.id
    JOIN organization_members om ON users.id = om.user_id
      AND users.organization_id = om.organization_id
  WHERE
    sess.id = $1
    AND sess.hash = $2
    AND expires_at > now()
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
    user_id AS actor_id,
    organization_id
  FROM
    base_lookup
UNION ALL
SELECT
  role_id AS actor_id,
  organization_id
FROM
  role_lookup
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
