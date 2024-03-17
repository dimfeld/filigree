SELECT
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
  _permission AS "_permission!: filigree::auth::ObjectPermission"
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
      organization_id = $2
      AND actor_id = ANY ($3)
      AND permission IN ('org_admin', 'PostImage::owner', 'PostImage::write', 'PostImage::read'))
	_permission ON _permission IS NOT NULL
WHERE
  tb.id = $1
  AND tb.organization_id = $2
