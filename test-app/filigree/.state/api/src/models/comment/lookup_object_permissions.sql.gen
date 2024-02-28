SELECT
  CASE WHEN bool_or(permission IN ('org_admin', 'Comment::owner')) THEN
    'owner'
  WHEN bool_or(permission = 'Comment::write') THEN
    'write'
  WHEN bool_or(permission = 'Comment::read') THEN
    'read'
  ELSE
    NULL
  END _permission
FROM
  permissions
WHERE
  organization_id = $1
  AND actor_id = ANY ($2)
  AND permission IN ('org_admin', 'Comment::owner', 'Comment::write', 'Comment::read')
