SELECT
  id AS "id: UserId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  name,
  password_hash,
  email,
  verified,
  _permission AS "_permission!: filigree::auth::ObjectPermission"
FROM
  users tb
  JOIN LATERAL (
    SELECT
      CASE WHEN bool_or(permission IN ('org_admin', 'User::owner')) THEN
        'owner'
      WHEN bool_or(permission = 'User::write') THEN
        'write'
      WHEN bool_or(permission = 'User::read') THEN
        'read'
      ELSE
        NULL
      END
    FROM
      permissions
    WHERE
      organization_id = $1
      AND actor_id = ANY ($2)
      AND permission IN ('org_admin', 'User::owner', 'User::write', 'User::read')
    GROUP BY
      permission) _permission ON _permission IS NOT NULL
WHERE
  AND organization_id = $1
ORDER BY
  < order_by >
LIMIT $3 OFFSET $4
