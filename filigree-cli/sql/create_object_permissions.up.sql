CREATE TABLE {{auth_schema}}.object_permissions (
  organization_id {{auth.id_sql_type}} NOT NULL {% if auth.builtin %}REFERENCES
    {{auth_schema}}.organizations (id) ON DELETE CASCADE{% endif %},
  actor_id {{auth.id_sql_type}} NOT NULL,
  object_id uuid NOT NULL,
  permission text NOT NULL,
  PRIMARY KEY (organization_id, actor_id, object_id, permission)
);
