WITH permissions AS (
  SELECT
    COALESCE(bool_or(permission IN ('org_admin', 'ReportSection::owner')), FALSE) AS is_owner,
    COALESCE(bool_or(permission IN ('org_admin', 'ReportSection::owner', 'ReportSection::write')), FALSE) AS is_user
  FROM
    permissions
  WHERE
    organization_id = $3
    AND actor_id = ANY ($4)
    AND permission IN ('org_admin', 'ReportSection::owner', 'ReportSection::write'))
UPDATE
  report_sections
SET
  name = $5,
  viz = $6,
  options = $7,
  updated_at = now()
FROM
  permissions
WHERE
  id = $1
  AND report_id = $2
  AND organization_id = $3
  AND (permissions.is_owner
    OR permissions.is_user)
