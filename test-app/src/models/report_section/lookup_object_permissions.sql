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
  AND permission IN ('org_admin', 'ReportSection::owner', 'ReportSection::write', 'ReportSection::read')
