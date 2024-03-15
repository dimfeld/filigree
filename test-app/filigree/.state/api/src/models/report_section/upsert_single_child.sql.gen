INSERT INTO report_sections (
  id,
  organization_id,
  name,
  viz,
  options,
  report_id)
VALUES (
  $1,
  $2,
  $3,
  $4,
  $5,
  $6)
ON CONFLICT (
  id)
  DO UPDATE SET
    name = EXCLUDED.name,
    viz = EXCLUDED.viz,
    options = EXCLUDED.options,
    report_id = EXCLUDED.report_id,
    updated_at = now()
  WHERE
    report_sections.organization_id = $2
    AND report_sections.report_id = EXCLUDED.report_id
  RETURNING
    id AS "id: ReportSectionId",
    organization_id AS "organization_id: crate::models::organization::OrganizationId",
    updated_at,
    created_at,
    name,
    viz,
    options,
    report_id AS "report_id: ReportId",
    'owner' AS "_permission!: filigree::auth::ObjectPermission"
