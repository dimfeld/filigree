INSERT INTO posts (
  id,
  organization_id,
  subject,
  body)
VALUES (
  $1,
  $2,
  $3,
  $4)
RETURNING
  id AS "id: PostId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  subject,
  body,
  'owner' AS "_permission!: filigree::auth::ObjectPermission"
