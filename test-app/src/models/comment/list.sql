SELECT
  id,
  organization_id,
  updated_at,
  created_at,
  body,
  post_id,
  perm._permission
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
      organization_id = $1
      AND actor_id = ANY ($2)
      AND permission IN ('org_admin', 'Comment::owner', 'Comment::write', 'Comment::read')) perm ON
	perm._permission IS NOT NULL
WHERE
  organization_id = $1
  AND __insertion_point_filters
ORDER BY
  __insertion_point_order_by
LIMIT $3 OFFSET $4
