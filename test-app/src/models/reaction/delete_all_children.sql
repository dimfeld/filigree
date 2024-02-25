DELETE FROM reactions
WHERE organization_id = $1
  AND post_id = $2
