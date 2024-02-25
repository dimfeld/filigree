SELECT
  CASE WHEN bool_or(permission IN ('org_admin', 'Reaction::owner')) THEN
    'owner'
  WHEN bool_or(permission = 'Reaction::write') THEN
    'write'
  WHEN bool_or(permission = 'Reaction::read') THEN
    'read'
  ELSE
    NULL
  END _permission
FROM
  permissions
WHERE
  organization_id = $1
  AND actor_id = ANY ($2)
  AND permission IN ('org_admin', 'Reaction::owner', 'Reaction::write', 'Reaction::read')
