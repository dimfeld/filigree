CREATE TABLE IF NOT EXISTS {{model_schema}}.delete_log (
  organization_id {{auth.id_sql_type}} NOT NULL,
  object_id uuid NOT NULL,
  object_type text NOT NULL,
  data jsonb NOT NULL,
  deleted_at timestamptz NOT NULL DEFAULT now()
);
