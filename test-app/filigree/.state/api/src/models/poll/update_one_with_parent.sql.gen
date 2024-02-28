WITH permissions AS (
  SELECT
    COALESCE(bool_or(permission IN ('org_admin', 'Poll::owner')), FALSE) AS is_owner,
    COALESCE(bool_or(permission IN ('org_admin', 'Poll::owner', 'Poll::write')), FALSE) AS is_user
  FROM
    permissions
  WHERE
    organization_id = $3
    AND actor_id = ANY ($4)
    AND permission IN ('org_admin', 'Poll::owner', 'Poll::write'))
UPDATE
  polls
SET
  question = $5,
  answers = $6,
  updated_at = now()
FROM
  permissions
WHERE
  id = $1
  AND post_id = $2
  AND organization_id = $3
  AND (permissions.is_owner
    OR permissions.is_user)
