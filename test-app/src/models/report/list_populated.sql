SELECT
  id,
  organization_id,
  updated_at,
  created_at,
  title,
  description,
  ui,
  (
    SELECT
      COALESCE(ARRAY_AGG(report_sections.id), ARRAY[]::uuid[])
    FROM
      report_sections
    WHERE
      report_id = tb.id
      AND organization_id = $1) AS "report_section_ids",
  perm._permission
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
      organization_id = $1
      AND actor_id = ANY ($2)
      AND permission IN ('org_admin', 'Report::owner', 'Report::write', 'Report::read')) perm ON
	perm._permission IS NOT NULL
WHERE
  organization_id = $1
  AND __insertion_point_filters
ORDER BY
  __insertion_point_order_by
LIMIT $3 OFFSET $4
