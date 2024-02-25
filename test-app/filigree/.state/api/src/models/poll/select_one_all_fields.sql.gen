SELECT
  id AS "id: PollId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  question,
  answers,
  post_id AS "post_id: PostId",
  _permission AS "_permission!: filigree::auth::ObjectPermission"
FROM
  polls tb
  JOIN LATERAL (
    SELECT
      CASE WHEN bool_or(permission IN ('org_admin', 'Poll::owner')) THEN
        'owner'
      WHEN bool_or(permission = 'Poll::write') THEN
        'write'
      WHEN bool_or(permission = 'Poll::read') THEN
        'read'
      ELSE
        NULL
      END _permission
    FROM
      permissions
    WHERE
      organization_id = $2
      AND actor_id = ANY ($3)
      AND permission IN ('org_admin', 'Poll::owner', 'Poll::write', 'Poll::read'))
	_permission ON _permission IS NOT NULL
WHERE
  tb.id = $1
  AND tb.organization_id = $2
