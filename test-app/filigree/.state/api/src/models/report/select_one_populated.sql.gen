SELECT
  id AS "id: ReportId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  title,
  description,
  ui,
  (
    SELECT
      COALESCE(ARRAY_AGG(JSONB_BUILD_OBJECT('id', id, 'organization_id', organization_id,
	'updated_at', updated_at, 'created_at', created_at, 'name', name,
	'viz', viz, 'options', options, 'report_id', report_id, '_permission',
	_permission)), ARRAY[]::jsonb[])
    FROM
      report_sections
    WHERE
      report_id = $1
      AND organization_id = $2) AS "report_sections!: Vec<ReportSection>",
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
      AND permission IN ('org_admin', 'Report::owner', 'Report::write', 'Report::read'))
	_permission ON _permission IS NOT NULL
WHERE
  tb.id = $1
  AND tb.organization_id = $2
