-- $1 = has owner permission on the model
-- $2 = organization_id
-- $3 = parent_id
INSERT INTO post_images (
  id,
  organization_id,
  post_id)
VALUES
  __insertion_point_insert_values
ON CONFLICT (
  id)
  DO UPDATE SET
    post_id = EXCLUDED.post_id,
    updated_at = now()
  WHERE
    post_images.organization_id = $2
    AND post_images.post_id = $3
  RETURNING
    id,
    organization_id,
    updated_at,
    created_at,
    file_storage_key,
    file_storage_bucket,
    file_original_name,
    file_size,
    file_hash,
    post_id,
    'owner' AS "_permission"
