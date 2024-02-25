-- $1 = has owner permission on the model
-- $2 = organization_id
-- $3 = parent_id
INSERT INTO comments (
  id,
  organization_id,
  body,
  post_id)
VALUES
  __insertion_point_insert_values
ON CONFLICT (
  id)
  DO UPDATE SET
    body = EXCLUDED.body,
    post_id = EXCLUDED.post_id,
    updated_at = now()
  WHERE
    comments.organization_id = $2
    AND comments.post_id = $3
  RETURNING
    id,
    organization_id,
    updated_at,
    created_at,
    body,
    post_id,
    'owner' AS "_permission"
