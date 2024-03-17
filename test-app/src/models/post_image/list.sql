SELECT
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
  perm._permission
FROM
  post_images tb
  JOIN LATERAL (
    SELECT
      CASE WHEN bool_or(permission IN ('org_admin', 'PostImage::owner')) THEN
        'owner'
      WHEN bool_or(permission = 'PostImage::write') THEN
        'write'
      WHEN bool_or(permission = 'PostImage::read') THEN
        'read'
      ELSE
        NULL
      END _permission
    FROM
      permissions
    WHERE
      organization_id = $1
      AND actor_id = ANY ($2)
      AND permission IN ('org_admin', 'PostImage::owner', 'PostImage::write', 'PostImage::read')) perm ON
	perm._permission IS NOT NULL
WHERE
  organization_id = $1
  AND __insertion_point_filters
ORDER BY
  __insertion_point_order_by
LIMIT $3 OFFSET $4
