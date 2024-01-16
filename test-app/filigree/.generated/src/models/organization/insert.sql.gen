INSERT INTO organizations (
  id,
  name,
  OWNER)
VALUES (
  $1,
  $2,
  $3)
RETURNING
  id AS "id: OrganizationId",
  updated_at,
  created_at,
  name,
  OWNER AS "owner: crate::models::user::UserId",
  active,
  'owner' AS "_permission!: filigree::auth::ObjectPermission"
