DELETE FROM reactions
WHERE organization_id = $1
  AND id = $2
  AND post_id = $3
