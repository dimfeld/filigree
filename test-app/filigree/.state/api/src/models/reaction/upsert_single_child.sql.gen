INSERT INTO reactions (
  id,
  organization_id,
  type,
  post_id)
VALUES (
  $1,
  $2,
  $3,
  $4)
ON CONFLICT (
  post_id)
  DO UPDATE SET type = EXCLUDED.type, post_id = EXCLUDED.post_id, updated_at = now()
  WHERE
    reactions.organization_id = $2
  RETURNING
    id AS "id: ReactionId",
    organization_id AS "organization_id: crate::models::organization::OrganizationId",
    updated_at,
    created_at,
    type AS "typ",
    post_id AS "post_id: PostId",
    'owner' AS "_permission!: filigree::auth::ObjectPermission"
