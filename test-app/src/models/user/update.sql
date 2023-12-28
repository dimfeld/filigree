WITH permissions AS (
  SELECT
    bool_or(permission IN ('org_admin', 'User::owner')) AS is_owner,
    bool_or(permission IN ('org_admin', 'User::owner', 'User::write')) AS is_user
  FROM
    permissions
  WHERE
    organization_id = $2
    AND actor_id = ANY ($3)
    AND permission IN ('org_admin', 'User::owner', 'User::write')
  GROUP BY
    permission)
UPDATE
  users
SET
  name = CASE WHEN permissions.is_owner THEN
    $4
  ELSE
    users.name
  END,
  email = CASE WHEN permissions.is_owner THEN
    $5
  ELSE
    users.email
  END,
  verified = CASE WHEN permissions.is_owner THEN
    $6
  ELSE
    users.verified
  END,
  updated_at = now()
FROM
  permissions
WHERE
  id = $1
  AND organization_id = $2
  AND (permissions.is_owner
    OR permissions.is_user)
