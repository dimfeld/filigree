SELECT
  id AS "id: ReportId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  title,
  description,
  ui,
  _permission AS "_permission!: filigree::auth::ObjectPermission"
FROM
  reports tb
  JOIN LATERAL (
    SELECT
      CASE WHEN bool_or(permission IN ('org_admin', 'Report::owner')) THEN
        'owner'
      WHEN bool_or(permission = 'Report::write') THEN
        'write'
      WHEN bool_or(permission = 'Report::read') THEN
        'read'
      ELSE
        NULL
      END _permission
    FROM
      permissions
    WHERE
      organization_id = $2
      AND actor_id = ANY ($3)
      AND permission IN ('org_admin', 'Report::owner', 'Report::write', 'Report::read')
    GROUP BY
      permission) _permission ON _permission IS NOT NULL
WHERE
  tb.id = $1
  AND tb.organization_id = $2
