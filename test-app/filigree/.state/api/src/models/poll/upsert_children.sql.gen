-- $1 = has owner permission on the model
-- $2 = organization_id
-- $3 = parent_id
INSERT INTO polls (
  id,
  organization_id,
  question,
  answers,
  post_id)
VALUES
  __insertion_point_insert_values
ON CONFLICT (
  id)
  DO UPDATE SET
    question = EXCLUDED.question,
    answers = EXCLUDED.answers,
    post_id = EXCLUDED.post_id,
    updated_at = now()
  WHERE
    polls.organization_id = $2
    AND polls.post_id = $3
  RETURNING
    id,
    organization_id,
    updated_at,
    created_at,
    question,
    answers,
    post_id,
    'owner' AS "_permission"
