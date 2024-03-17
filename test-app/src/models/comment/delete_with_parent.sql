DELETE FROM comments
WHERE organization_id = $1
  AND post_id = $2
  AND id = $3
