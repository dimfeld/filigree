SELECT
  id AS "id: ReportSectionId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  name,
  viz,
  options,
  report_id AS "report_id: ReportId",
  _permission AS "_permission!: filigree::auth::ObjectPermission"
FROM
  report_sections tb
  JOIN LATERAL (
    SELECT
      CASE WHEN bool_or(permission IN ('org_admin', 'ReportSection::owner')) THEN
        'owner'
      WHEN bool_or(permission = 'ReportSection::write') THEN
        'write'
      WHEN bool_or(permission = 'ReportSection::read') THEN
        'read'
      ELSE
        NULL
      END _permission
    FROM
      permissions
    WHERE
      organization_id = $2
      AND actor_id = ANY ($3)
      AND permission IN ('org_admin', 'ReportSection::owner', 'ReportSection::write', 'ReportSection::read'))
	_permission ON _permission IS NOT NULL
WHERE
  tb.id = $1
  AND tb.organization_id = $2
