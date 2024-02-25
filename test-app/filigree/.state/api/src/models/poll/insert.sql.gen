INSERT INTO polls (
  id,
  organization_id,
  question,
  answers,
  post_id)
VALUES (
  $1,
  $2,
  $3,
  $4,
  $5)
RETURNING
  id AS "id: PollId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  question,
  answers,
  post_id AS "post_id: PostId",
  'owner' AS "_permission!: filigree::auth::ObjectPermission"
