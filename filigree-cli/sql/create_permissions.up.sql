CREATE TABLE permissions (
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  -- user or role
  actor_id uuid NOT NULL,
  permission text NOT NULL,
  PRIMARY KEY (organization_id, actor_id, permission)
);
