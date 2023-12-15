CREATE TABLE delete_log (
  team_id uuid NOT NULL,
  object_id uuid NOT NULL,
  object_type text NOT NULL,
  data jsonb NOT NULL,
  deleted_at timestamptz NOT NULL DEFAULT now(),
);
