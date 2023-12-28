WITH permissions AS (
  SELECT
    bool_or(permission IN ('org_admin', 'Report::owner')) AS is_owner,
    bool_or(permission IN ('org_admin', 'Report::owner', 'Report::write')) AS is_user
  FROM
    permissions
  WHERE
    organization_id = $2
    AND actor_id = ANY ($3)
    AND permission IN ('org_admin', 'Report::owner', 'Report::write')
  GROUP BY
    permission)
UPDATE
  reports
SET
  title = $4,
  description = $5,
  ui = CASE WHEN permissions.is_owner THEN
    $6
  ELSE
    reports.ui
  END,
  updated_at = now()
FROM
  permissions
WHERE
  id = $1
  AND organization_id = $2
  AND (permissions.is_owner
    OR permissions.is_user)
