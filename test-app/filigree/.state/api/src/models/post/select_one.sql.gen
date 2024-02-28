SELECT
  id AS "id: PostId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  subject,
  body,
  _permission AS "_permission!: filigree::auth::ObjectPermission"
FROM
  posts tb
  JOIN LATERAL (
    SELECT
      CASE WHEN bool_or(permission IN ('org_admin', 'Post::owner')) THEN
        'owner'
      WHEN bool_or(permission = 'Post::write') THEN
        'write'
      WHEN bool_or(permission = 'Post::read') THEN
        'read'
      ELSE
        NULL
      END _permission
    FROM
      permissions
    WHERE
      organization_id = $2
      AND actor_id = ANY ($3)
      AND permission IN ('org_admin', 'Post::owner', 'Post::write', 'Post::read'))
	_permission ON _permission IS NOT NULL
WHERE
  tb.id = $1
  AND tb.organization_id = $2
