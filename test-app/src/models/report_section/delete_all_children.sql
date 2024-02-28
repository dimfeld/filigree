DELETE FROM report_sections
WHERE organization_id = $1
  AND report_id = $2
