SELECT
  id,
  organization_id,
  updated_at,
  created_at,
  subject,
  body,
  (
    SELECT
      COALESCE(ARRAY_AGG(comments.id), ARRAY[]::uuid[])
    FROM
      comments
    WHERE
      post_id = tb.id
      AND organization_id = $1) AS "comment_ids",
  (
    SELECT
      polls.id
    FROM
      polls
    WHERE
      post_id = tb.id
      AND organization_id = $1
    LIMIT 1) AS "poll_id",
perm._permission
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
      organization_id = $1
      AND actor_id = ANY ($2)
      AND permission IN ('org_admin', 'Post::owner', 'Post::write', 'Post::read')) perm ON
	perm._permission IS NOT NULL
WHERE
  organization_id = $1
  AND __insertion_point_filters
ORDER BY
  __insertion_point_order_by
LIMIT $3 OFFSET $4
