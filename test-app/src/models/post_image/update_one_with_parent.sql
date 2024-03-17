WITH permissions AS (
  SELECT
    COALESCE(bool_or(permission IN ('org_admin', 'PostImage::owner')), FALSE) AS is_owner,
    COALESCE(bool_or(permission IN ('org_admin', 'PostImage::owner', 'PostImage::write')), FALSE) AS is_user
  FROM
    permissions
  WHERE
    organization_id = $3
    AND actor_id = ANY ($4)
    AND permission IN ('org_admin', 'PostImage::owner', 'PostImage::write'))
UPDATE
  post_images
SET
  file_storage_key = $5,
  file_storage_bucket = $6,
  file_original_name = $7,
  file_size = $8,
  file_hash = $9,
  updated_at = now()
FROM
  permissions
WHERE
  id = $1
  AND post_id = $2
  AND organization_id = $3
  AND (permissions.is_owner
    OR permissions.is_user)
