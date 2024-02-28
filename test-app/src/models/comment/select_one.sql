SELECT
  id AS "id: CommentId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  body,
  post_id AS "post_id: PostId",
  _permission AS "_permission!: filigree::auth::ObjectPermission"
FROM
  comments tb
  JOIN LATERAL (
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
      organization_id = $2
      AND actor_id = ANY ($3)
      AND permission IN ('org_admin', 'Comment::owner', 'Comment::write', 'Comment::read'))
	_permission ON _permission IS NOT NULL
WHERE
  tb.id = $1
  AND tb.organization_id = $2
