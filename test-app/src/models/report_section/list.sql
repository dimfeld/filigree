SELECT
  id,
  organization_id,
  updated_at,
  created_at,
  name,
  viz,
  options,
  report_id,
  perm._permission
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
      organization_id = $1
      AND actor_id = ANY ($2)
      AND permission IN ('org_admin', 'ReportSection::owner', 'ReportSection::write', 'ReportSection::read')) perm ON
	perm._permission IS NOT NULL
WHERE
  organization_id = $1
  AND __insertion_point_filters
ORDER BY
  __insertion_point_order_by
LIMIT $3 OFFSET $4
