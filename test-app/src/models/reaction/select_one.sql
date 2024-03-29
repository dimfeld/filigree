SELECT
  id AS "id: ReactionId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  type AS "typ",
  post_id AS "post_id: PostId",
  _permission AS "_permission!: filigree::auth::ObjectPermission"
FROM
  reactions tb
  JOIN LATERAL (
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
      organization_id = $2
      AND actor_id = ANY ($3)
      AND permission IN ('org_admin', 'Reaction::owner', 'Reaction::write', 'Reaction::read'))
	_permission ON _permission IS NOT NULL
WHERE
  tb.id = $1
  AND tb.organization_id = $2
