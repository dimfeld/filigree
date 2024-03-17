SELECT
  CASE WHEN bool_or(permission IN ('org_admin', 'PostImage::owner')) THEN
    'owner'
  WHEN bool_or(permission = 'PostImage::write') THEN
    'write'
  WHEN bool_or(permission = 'PostImage::read') THEN
    'read'
  ELSE
    NULL
  END _permission
FROM
  permissions
WHERE
  organization_id = $1
  AND actor_id = ANY ($2)
  AND permission IN ('org_admin', 'PostImage::owner', 'PostImage::write', 'PostImage::read')
