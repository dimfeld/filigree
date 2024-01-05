INSERT INTO users (
  id,
  organization_id,
  name,
  email,
  verified)
VALUES (
  $1,
  $2,
  $3,
  $4,
  $5)
RETURNING
  id AS "id: UserId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  name,
  password_hash,
  email,
  verified,
  'owner' AS "_permission!: filigree::auth::ObjectPermission"
