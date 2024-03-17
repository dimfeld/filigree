INSERT INTO post_images (
  id,
  organization_id,
  file_storage_key,
  file_storage_bucket,
  file_original_name,
  file_size,
  file_hash,
  post_id)
VALUES (
  $1,
  $2,
  $3,
  $4,
  $5,
  $6,
  $7,
  $8)
ON CONFLICT (
  post_id)
  DO UPDATE SET
    file_storage_key = EXCLUDED.file_storage_key,
    file_storage_bucket = EXCLUDED.file_storage_bucket,
    file_original_name = EXCLUDED.file_original_name,
    file_size = EXCLUDED.file_size,
    file_hash = EXCLUDED.file_hash,
    post_id = EXCLUDED.post_id,
    updated_at = now()
  WHERE
    post_images.organization_id = $2
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
