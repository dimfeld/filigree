INSERT INTO post_images (
  id,
  organization_id,
  post_id)
VALUES (
  $1,
  $2,
  $3)
RETURNING
  id AS "id: PostImageId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  file_storage_key,
  file_storage_bucket,
  file_original_name,
  file_size,
  file_hash,
  post_id AS "post_id: PostId",
  'owner' AS "_permission!: filigree::auth::ObjectPermission"
