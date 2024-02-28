INSERT INTO comments (
  id,
  organization_id,
  body,
  post_id)
VALUES (
  $1,
  $2,
  $3,
  $4)
RETURNING
  id AS "id: CommentId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  body,
  post_id AS "post_id: PostId",
  'owner' AS "_permission!: filigree::auth::ObjectPermission"
