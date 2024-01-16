INSERT INTO reports (
  id,
  organization_id,
  title,
  description,
  ui)
VALUES (
  $1,
  $2,
  $3,
  $4,
  $5)
RETURNING
  id AS "id: ReportId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  title,
  description,
  ui,
  'owner' AS "_permission!: filigree::auth::ObjectPermission"
