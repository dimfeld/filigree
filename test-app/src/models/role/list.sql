SELECT
  id AS "id: RoleId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  name,
  description,
  _permission AS "_permission!: filigree::auth::ObjectPermission"
FROM
  roles tb
  JOIN LATERAL (
    SELECT
      CASE WHEN bool_or(permission IN ('org_admin', 'Role::owner')) THEN
        'owner'
      WHEN bool_or(permission = 'Role::write') THEN
        'write'
      WHEN bool_or(permission = 'Role::read') THEN
        'read'
      ELSE
        NULL
      END
    FROM
      permissions
    WHERE
      organization_id = $1
      AND actor_id = ANY ($2)
      AND permission IN ('org_admin', 'Role::owner', 'Role::write', 'Role::read')
    GROUP BY
      permission) _permission ON _permission IS NOT NULL
WHERE
  AND organization_id = $1
ORDER BY
  < order_by >
LIMIT $3 OFFSET $4
