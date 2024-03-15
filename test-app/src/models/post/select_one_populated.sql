SELECT
  id AS "id: PostId",
  organization_id AS "organization_id: crate::models::organization::OrganizationId",
  updated_at,
  created_at,
  subject,
  body,
  (
    SELECT
      COALESCE(ARRAY_AGG(comments.id), ARRAY[]::uuid[])
    FROM
      comments
    WHERE
      post_id = $1
      AND organization_id = $2) AS "comment_ids!: Vec<CommentId>",
  (
    SELECT
      COALESCE(ARRAY_AGG(JSONB_BUILD_OBJECT('id', id, 'organization_id', organization_id,
	'updated_at', updated_at, 'created_at', created_at, 'typ', type,
	'post_id', post_id, '_permission', _permission)), ARRAY[]::jsonb[])
    FROM
      reactions
    WHERE
      post_id = $1
      AND organization_id = $2) AS "reactions!: Vec<Reaction>",
  (
    SELECT
      JSONB_BUILD_OBJECT('id', id, 'organization_id', organization_id, 'updated_at',
	updated_at, 'created_at', created_at, 'question', question, 'answers',
	answers, 'post_id', post_id, '_permission', _permission)
    FROM
      polls
    WHERE
      post_id = $1
      AND organization_id = $2
    LIMIT 1) AS "poll: Poll",
(
  SELECT
    COALESCE(ARRAY_AGG(JSONB_BUILD_OBJECT('id', id, 'organization_id', organization_id,
      'updated_at', updated_at, 'created_at', created_at, 'file_storage_key', file_storage_key,
      'file_storage_bucket', file_storage_bucket, 'file_original_name', file_original_name, 'file_size',
      file_size, 'file_hash', file_hash, 'post_id', post_id, '_permission',
      _permission)), ARRAY[]::jsonb[])
  FROM
    post_images
  WHERE
    post_id = $1
    AND organization_id = $2) AS "images!: Vec<PostImage>",
_permission AS "_permission!: filigree::auth::ObjectPermission"
FROM
  posts tb
  JOIN LATERAL (
    SELECT
      CASE WHEN bool_or(permission IN ('org_admin', 'Post::owner')) THEN
        'owner'
      WHEN bool_or(permission = 'Post::write') THEN
        'write'
      WHEN bool_or(permission = 'Post::read') THEN
        'read'
      ELSE
        NULL
      END _permission
    FROM
      permissions
    WHERE
      organization_id = $2
      AND actor_id = ANY ($3)
      AND permission IN ('org_admin', 'Post::owner', 'Post::write', 'Post::read'))
	_permission ON _permission IS NOT NULL
WHERE
  tb.id = $1
  AND tb.organization_id = $2
