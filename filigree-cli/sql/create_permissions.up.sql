CREATE TABLE {{auth_schema}}.permissions (
  organization_id {{auth.id_sql_type}} NOT NULL {% if auth.builtin %}REFERENCES
    {{auth_schema}}.organizations (id) ON DELETE CASCADE{% endif %},
  -- user or role
  actor_id {{auth.id_sql_type}} NOT NULL,
  permission text NOT NULL,
  PRIMARY KEY (organization_id, actor_id, permission)
);
