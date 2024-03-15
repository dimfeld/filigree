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
ON CONFLICT (
  id)
  DO UPDATE SET
    body = EXCLUDED.body,
    post_id = EXCLUDED.post_id,
    updated_at = now()
  WHERE
    comments.organization_id = $2
    AND comments.post_id = EXCLUDED.post_id
  RETURNING
    id AS "id: CommentId",
    organization_id AS "organization_id: crate::models::organization::OrganizationId",
    updated_at,
    created_at,
    body,
    post_id AS "post_id: PostId",
    'owner' AS "_permission!: filigree::auth::ObjectPermission"
